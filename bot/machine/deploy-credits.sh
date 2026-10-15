#!/bin/bash
# Deploys credits.py as the AWS Lambda that opens an issue when the project's credits run low, run
# every six hours by an EventBridge rule (README.md, "The credits watch"). Idempotent: run it again
# after changing credits.py. Needs the AWS CLI signed in to the project, in the project's Region.
#
#   REPO=OWNER/NAME CREDITS_GITHUB_TOKEN=<token> bot/machine/deploy-credits.sh
#
# CREDITS_GITHUB_TOKEN is a fine-grained token for the repository with Issues read and write and
# nothing else. Without it, a function already deployed keeps its token, and a new one only logs
# until it has one (set GITHUB_TOKEN in the Lambda console, or deploy again with it).
set -euo pipefail
REPO="${REPO:?the repository: OWNER/NAME}"
export AWS_REGION="${AWS_REGION:-us-east-2}"
NAME="jackioh-credits-watch"
here="$(cd "$(dirname "$0")" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

account="$(aws sts get-caller-identity --query Account --output text)"
role_arn="arn:aws:iam::${account}:role/${NAME}"

# The function's role: its logs and reading the plan's credits, nothing else.
if ! aws iam get-role --role-name "$NAME" >/dev/null 2>&1; then
  aws iam create-role --role-name "$NAME" --assume-role-policy-document '{"Version":"2012-10-17",
    "Statement":[{"Effect":"Allow","Principal":{"Service":"lambda.amazonaws.com"},
    "Action":"sts:AssumeRole"}]}' >/dev/null
  aws iam attach-role-policy --role-name "$NAME" \
    --policy-arn arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole
  sleep 10  # a new role takes a moment before Lambda can assume it
fi
aws iam put-role-policy --role-name "$NAME" --policy-name read-the-credits \
  --policy-document '{"Version":"2012-10-17","Statement":[
    {"Effect":"Allow","Action":"freetier:GetAccountPlanState","Resource":"*"}]}'

cp "$here/credits.py" "$work/credits.py"
(cd "$work" && zip -q credits.zip credits.py)
token="${CREDITS_GITHUB_TOKEN:-}"
if aws lambda get-function --function-name "$NAME" >/dev/null 2>&1; then
  [ -n "$token" ] || token="$(aws lambda get-function-configuration --function-name "$NAME" \
    --query Environment.Variables.GITHUB_TOKEN --output text)"
  [ "$token" != "None" ] || token=""
  env="Variables={REPO=${REPO}${token:+,GITHUB_TOKEN=${token}}}"
  aws lambda update-function-code --function-name "$NAME" \
    --zip-file "fileb://$work/credits.zip" >/dev/null
  aws lambda wait function-updated --function-name "$NAME"
  aws lambda update-function-configuration --function-name "$NAME" --environment "$env" \
    >/dev/null
  aws lambda wait function-updated --function-name "$NAME"
else
  env="Variables={REPO=${REPO}${token:+,GITHUB_TOKEN=${token}}}"
  aws lambda create-function --function-name "$NAME" --runtime python3.13 \
    --handler credits.handler --role "$role_arn" --zip-file "fileb://$work/credits.zip" \
    --timeout 60 --memory-size 256 --environment "$env" \
    --description "Opens a JackiOh issue when the AWS project's credits run low" >/dev/null
  aws lambda wait function-active --function-name "$NAME"
fi
aws logs create-log-group --log-group-name "/aws/lambda/$NAME" >/dev/null 2>&1 || true
aws logs put-retention-policy --log-group-name "/aws/lambda/$NAME" --retention-in-days 7

rule_arn="$(aws events put-rule --name "$NAME" --schedule-expression "rate(6 hours)" \
  --description "Check the AWS project's credits" --query RuleArn --output text)"
aws lambda add-permission --function-name "$NAME" --statement-id every-six-hours \
  --action lambda:InvokeFunction --principal events.amazonaws.com --source-arn "$rule_arn" \
  >/dev/null 2>&1 || true
function_arn="$(aws lambda get-function --function-name "$NAME" \
  --query Configuration.FunctionArn --output text)"
aws events put-targets --rule "$NAME" --targets "Id=credits,Arn=${function_arn}" >/dev/null

aws lambda invoke --function-name "$NAME" "$work/out.json" >/dev/null
echo "deployed $NAME; a test call returned: $(cat "$work/out.json")"
