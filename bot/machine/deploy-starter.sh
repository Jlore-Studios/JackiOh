#!/bin/bash
# Deploys starter.py as the AWS Lambda that starts the machine when a job waits for it, run every
# five minutes by an EventBridge rule (README.md, "The starter"). Idempotent: run it again after
# changing starter.py. Needs the AWS CLI signed in to the project, in the machine's Region.
#
#   REPO_ID=$(gh api repos/OWNER/REPO --jq .id) bot/machine/deploy-starter.sh
#
# STARTER_GITHUB_TOKEN, if set, is handed to the function to raise GitHub's rate limit (a
# fine-grained token with read access to Actions only); without it the public API is used.
set -euo pipefail
REPO_ID="${REPO_ID:?the numeric id of the repository: gh api repos/OWNER/REPO --jq .id}"
export AWS_REGION="${AWS_REGION:-us-east-2}"
# The machine: INSTANCE_ID, or the one instance tagged Name=jackioh-night-vm.
if [ -z "${INSTANCE_ID:-}" ]; then
  INSTANCE_ID="$(aws ec2 describe-instances --filters Name=tag:Name,Values=jackioh-night-vm \
    Name=instance-state-name,Values=pending,running,stopping,stopped \
    --query 'Reservations[].Instances[].InstanceId' --output text)"
fi
if ! [[ "$INSTANCE_ID" =~ ^i-[0-9a-f]+$ ]]; then
  echo "no single instance tagged Name=jackioh-night-vm; set INSTANCE_ID" >&2
  exit 1
fi
NAME=jackioh-night-vm-starter
ROLE=jackioh-night-vm-starter
here="$(cd "$(dirname "$0")" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

account="$(aws sts get-caller-identity --query Account --output text)"
role_arn="arn:aws:iam::${account}:role/${ROLE}"

# The function's role: its logs, describing instances, and starting this one instance only.
if ! aws iam get-role --role-name "$ROLE" >/dev/null 2>&1; then
  aws iam create-role --role-name "$ROLE" --assume-role-policy-document '{"Version":"2012-10-17",
    "Statement":[{"Effect":"Allow","Principal":{"Service":"lambda.amazonaws.com"},
    "Action":"sts:AssumeRole"}]}' >/dev/null
  aws iam attach-role-policy --role-name "$ROLE" \
    --policy-arn arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole
  sleep 10  # a new role takes a moment before Lambda can assume it
fi
aws iam put-role-policy --role-name "$ROLE" --policy-name start-the-night-machine \
  --policy-document "{\"Version\":\"2012-10-17\",\"Statement\":[
    {\"Effect\":\"Allow\",\"Action\":\"ec2:StartInstances\",
     \"Resource\":\"arn:aws:ec2:${AWS_REGION}:${account}:instance/${INSTANCE_ID}\"},
    {\"Effect\":\"Allow\",\"Action\":\"ec2:DescribeInstances\",\"Resource\":\"*\"}]}"

# 256 MB: a call peaks at about 107 MB (boto3 is most of it), too close to 128.
cp "$here/starter.py" "$work/starter.py"
(cd "$work" && zip -q starter.zip starter.py)
token="${STARTER_GITHUB_TOKEN:-}"
env="Variables={INSTANCE_ID=${INSTANCE_ID},REPO_ID=${REPO_ID}${token:+,GITHUB_TOKEN=${token}}}"
if aws lambda get-function --function-name "$NAME" >/dev/null 2>&1; then
  aws lambda update-function-code --function-name "$NAME" \
    --zip-file "fileb://$work/starter.zip" >/dev/null
  aws lambda wait function-updated --function-name "$NAME"
  aws lambda update-function-configuration --function-name "$NAME" --environment "$env" \
    --memory-size 256 >/dev/null
  aws lambda wait function-updated --function-name "$NAME"
else
  aws lambda create-function --function-name "$NAME" --runtime python3.13 \
    --handler starter.handler --role "$role_arn" --zip-file "fileb://$work/starter.zip" \
    --timeout 60 --memory-size 256 --environment "$env" \
    --description "Starts the JackiOh night machine when a job waits for its runners" >/dev/null
  aws lambda wait function-active --function-name "$NAME"
fi
aws logs create-log-group --log-group-name "/aws/lambda/$NAME" >/dev/null 2>&1 || true
aws logs put-retention-policy --log-group-name "/aws/lambda/$NAME" --retention-in-days 7

# Every five minutes. A rule's target needs only a permission on the function, no role.
rule_arn="$(aws events put-rule --name "$NAME" --schedule-expression "rate(5 minutes)" \
  --description "Look for night-bot jobs waiting for the machine" --query RuleArn --output text)"
aws lambda add-permission --function-name "$NAME" --statement-id every-five-minutes \
  --action lambda:InvokeFunction --principal events.amazonaws.com --source-arn "$rule_arn" \
  >/dev/null 2>&1 || true
function_arn="$(aws lambda get-function --function-name "$NAME" \
  --query Configuration.FunctionArn --output text)"
aws events put-targets --rule "$NAME" --targets "Id=starter,Arn=${function_arn}" >/dev/null

aws lambda invoke --function-name "$NAME" "$work/out.json" >/dev/null
echo "deployed $NAME; a test call returned: $(cat "$work/out.json")"
