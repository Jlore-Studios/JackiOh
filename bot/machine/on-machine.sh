#!/bin/bash
# Runs a local script on the machine as root through Session Manager, with the scripts beside it,
# and prints its output.
# The machine has no inbound ports and no key pair; this is the way in for scripts (an
# interactive shell is `aws ssm start-session --target <instance>`).
#
#   bot/machine/on-machine.sh bot/machine/setup.sh gpt agy muse devin
#
# Starts the machine first if it is stopped. Needs the AWS CLI signed in to the project.
set -euo pipefail
export AWS_REGION="${AWS_REGION:-us-east-2}"
# The machine: INSTANCE_ID, or the one instance tagged Name=$TAG (the training box
# takes TAG=jackioh-train-box).
TAG="${TAG:-jackioh-night-vm}"
if [ -z "${INSTANCE_ID:-}" ]; then
  INSTANCE_ID="$(aws ec2 describe-instances --filters Name=tag:Name,Values="$TAG" \
    Name=instance-state-name,Values=pending,running,stopping,stopped \
    --query 'Reservations[].Instances[].InstanceId' --output text)"
fi
if ! [[ "$INSTANCE_ID" =~ ^i-[0-9a-f]+$ ]]; then
  echo "no single instance tagged Name=$TAG; set INSTANCE_ID" >&2
  exit 1
fi
script="${1:?a script to run}"
shift

state="$(aws ec2 describe-instances --instance-ids "$INSTANCE_ID" \
  --query 'Reservations[0].Instances[0].State.Name' --output text)"
if [ "$state" != running ]; then
  [ "$state" = stopped ] || aws ec2 wait instance-stopped --instance-ids "$INSTANCE_ID"
  aws ec2 start-instances --instance-ids "$INSTANCE_ID" >/dev/null
  aws ec2 wait instance-running --instance-ids "$INSTANCE_ID"
  echo "started $INSTANCE_ID; waiting for its Session Manager agent" >&2
  for _ in $(seq 1 30); do
    online="$(aws ssm describe-instance-information \
      --filters "Key=InstanceIds,Values=$INSTANCE_ID" \
      --query 'InstanceInformationList[0].PingStatus' --output text)"
    [ "$online" = Online ] && break
    sleep 10
  done
fi

# The script travels with the scripts beside it (setup.sh installs clean.sh), as a base64
# tarball, and its arguments shell-quoted.
body="$(COPYFILE_DISABLE=1 tar czf - -C "$(dirname "$script")" --exclude='*.md' --exclude='*.py' --exclude=__pycache__ . \
  | base64 | tr -d '\n')"
quoted=""
for arg in "$@"; do quoted+=" $(printf '%q' "$arg")"; done
name="$(printf '%q' "$(basename "$script")")"
command="rm -rf /root/on-machine && mkdir -p /root/on-machine && echo $body | base64 -d | tar xzf - -C /root/on-machine && bash /root/on-machine/$name$quoted; status=\$?; rm -rf /root/on-machine; exit \$status"
params="$(jq -cn --arg c "$command" '{commands: [$c], executionTimeout: ["3600"]}')"
id="$(aws ssm send-command --instance-ids "$INSTANCE_ID" --document-name AWS-RunShellScript \
  --comment "$(basename "$script")" --timeout-seconds 600 --parameters "$params" \
  --query Command.CommandId --output text)"
while :; do
  sleep 5
  status="$(aws ssm get-command-invocation --command-id "$id" --instance-id "$INSTANCE_ID" \
    --query Status --output text 2>/dev/null || echo Pending)"
  case "$status" in Pending|InProgress|Delayed) continue ;; esac
  break
done
aws ssm get-command-invocation --command-id "$id" --instance-id "$INSTANCE_ID" \
  --query StandardOutputContent --output text
errors="$(aws ssm get-command-invocation --command-id "$id" --instance-id "$INSTANCE_ID" \
  --query StandardErrorContent --output text)"
[ -z "$errors" ] || echo "$errors" >&2
[ "$status" = Success ]
