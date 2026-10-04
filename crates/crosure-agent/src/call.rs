//! One tool call: profile and permission checks, approval, execution, and
//! the text the model gets back.

use serde_json::Value;

use crate::agent::Executor;
use crate::policy::{ApprovalRequest, Permission, Permissions};
use crate::render::render_result;
use crate::tools::parse_tool_call;
use crate::{AgentEvent, Profile, Sink};

pub(crate) struct CallCtx<'a> {
    pub(crate) exec: &'a dyn Executor,
    pub(crate) sink: &'a dyn Sink,
    pub(crate) by: String,
    pub(crate) profile: Profile,
    pub(crate) permissions: &'a Permissions,
}

fn failed(
    ctx: &CallCtx<'_>,
    name: &str,
    command: String,
    why: String,
    error: String,
) -> (String, bool) {
    ctx.sink.emit(AgentEvent::ToolCall {
        tool: name.into(),
        command,
        why,
        step_id: None,
        summary: None,
        error: Some(error.clone()),
    });
    (format!("Error: {error}"), true)
}

pub(crate) fn run_one(ctx: &CallCtx<'_>, id: &str, name: &str, input: &Value) -> (String, bool) {
    if !ctx.profile.allows(name) {
        return failed(
            ctx,
            name,
            name.into(),
            String::new(),
            format!("`{name}` is not available in this profile"),
        );
    }
    let call = match parse_tool_call(name, input) {
        Ok(c) => c,
        Err(e) => {
            return failed(
                ctx,
                name,
                name.into(),
                String::new(),
                format!("invalid input: {e}"),
            )
        }
    };
    let command = call.op.command();
    match ctx.permissions.get(name) {
        Permission::Allow => {}
        Permission::Deny => {
            return failed(
                ctx,
                name,
                command,
                call.why,
                "blocked by tool permissions".into(),
            )
        }
        Permission::Confirm => {
            let request = ApprovalRequest {
                id: id.into(),
                tool: name.into(),
                command: command.clone(),
                why: call.why.clone(),
            };
            ctx.sink.emit(AgentEvent::ApprovalRequested {
                request: request.clone(),
            });
            let allowed = ctx.sink.approve(&request);
            ctx.sink.emit(AgentEvent::ApprovalResolved {
                id: id.into(),
                allowed,
            });
            if !allowed {
                return failed(
                    ctx,
                    name,
                    command,
                    call.why,
                    "the analyst declined this action".into(),
                );
            }
        }
    }
    match ctx.exec.execute(&call, &ctx.by) {
        Ok(done) => {
            ctx.sink.emit(AgentEvent::ToolCall {
                tool: name.into(),
                command: done.command.clone(),
                why: call.why.clone(),
                step_id: Some(done.step_id.clone()),
                summary: Some(done.summary.clone()),
                error: None,
            });
            (
                render_result(&done.kind, &done.summary, &done.result, call.op.offset()),
                false,
            )
        }
        Err(e) => failed(ctx, name, command, call.why, e),
    }
}
