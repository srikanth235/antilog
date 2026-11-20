"""Spend kill switch. Called by a CloudWatch alarm; safe to call again and again.

An alarm event, or an empty one, locks everything:
  1. an inline deny-all policy on each locked role (not this function's own role),
  2. the server function's reserved concurrency set to 0, so nothing runs.

{"action": "release"} undoes it: deletes the deny-all policies and deletes the concurrency
setting (the next deploy applies the configured one).
Only boto3, which Lambda's Python runtime already has.
"""

import json
import os

import boto3

POLICY = "antilog-killswitch-deny"
DENY_ALL = json.dumps(
    {"Version": "2012-10-17", "Statement": [{"Effect": "Deny", "Action": "*", "Resource": "*"}]}
)

ROLES = [r for r in os.environ.get("LOCKED_ROLES", "").split(",") if r]
SERVER = os.environ.get("SERVER", "")

iam = boto3.client("iam")
lam = boto3.client("lambda")


def lock():
    done = {"roles": [], "concurrency": None}
    for role in ROLES:
        iam.put_role_policy(RoleName=role, PolicyName=POLICY, PolicyDocument=DENY_ALL)
        done["roles"].append(role)
    lam.put_function_concurrency(FunctionName=SERVER, ReservedConcurrentExecutions=0)
    done["concurrency"] = 0
    return done


def release():
    done = {"roles": [], "concurrency": "removed"}
    for role in ROLES:
        try:
            iam.delete_role_policy(RoleName=role, PolicyName=POLICY)
        except iam.exceptions.NoSuchEntityException:
            pass
        done["roles"].append(role)
    lam.delete_function_concurrency(FunctionName=SERVER)
    return done


def handler(event, context):
    action = event.get("action") if isinstance(event, dict) else None
    print(json.dumps({"kill_switch": action or "lock", "event": event}, default=str))
    result = release() if action == "release" else lock()
    print(json.dumps(result))
    return result
