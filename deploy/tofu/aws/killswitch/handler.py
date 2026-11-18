"""Spend kill switch. Called by a CloudWatch alarm; safe to call again and again.

An alarm event, or an empty one, locks everything:
  1. an inline deny-all policy on each locked role (not this function's own role),
  2. the server function's reserved concurrency set to 0, so nothing runs,
  3. every enabled schedule in the wake-up group set to DISABLED.

{"action": "release"} undoes it: deletes the deny-all policies, deletes the concurrency
setting (the next deploy applies the configured one) and enables the schedules again.
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
GROUP = os.environ.get("SCHEDULE_GROUP", "")

iam = boto3.client("iam")
lam = boto3.client("lambda")
scheduler = boto3.client("scheduler")

# What UpdateSchedule needs back, because it replaces the whole schedule.
KEEP = (
    "Name",
    "GroupName",
    "ScheduleExpression",
    "ScheduleExpressionTimezone",
    "FlexibleTimeWindow",
    "Target",
    "ActionAfterCompletion",
    "Description",
    "StartDate",
    "EndDate",
    "KmsKeyArn",
)


def set_schedules(from_state, to_state):
    changed = []
    pages = scheduler.get_paginator("list_schedules").paginate(GroupName=GROUP, State=from_state)
    for page in pages:
        for item in page["Schedules"]:
            current = scheduler.get_schedule(Name=item["Name"], GroupName=GROUP)
            update = {k: current[k] for k in KEEP if k in current}
            scheduler.update_schedule(State=to_state, **update)
            changed.append(item["Name"])
    return changed


def lock():
    done = {"roles": [], "concurrency": None, "schedules": []}
    for role in ROLES:
        iam.put_role_policy(RoleName=role, PolicyName=POLICY, PolicyDocument=DENY_ALL)
        done["roles"].append(role)
    lam.put_function_concurrency(FunctionName=SERVER, ReservedConcurrentExecutions=0)
    done["concurrency"] = 0
    done["schedules"] = set_schedules("ENABLED", "DISABLED")
    return done


def release():
    done = {"roles": [], "concurrency": "removed", "schedules": []}
    for role in ROLES:
        try:
            iam.delete_role_policy(RoleName=role, PolicyName=POLICY)
        except iam.exceptions.NoSuchEntityException:
            pass
        done["roles"].append(role)
    lam.delete_function_concurrency(FunctionName=SERVER)
    done["schedules"] = set_schedules("DISABLED", "ENABLED")
    return done


def handler(event, context):
    action = event.get("action") if isinstance(event, dict) else None
    print(json.dumps({"kill_switch": action or "lock", "event": event}, default=str))
    result = release() if action == "release" else lock()
    print(json.dumps(result))
    return result
