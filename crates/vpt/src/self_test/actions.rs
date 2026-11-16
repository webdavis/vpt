use super::plan::Run;
use crate::report::Row;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use vpt_adapters::{ActionSend, SendOutcome, SystemClock, WithdrawOutcome};
use vpt_adapters::{send_action, withdraw_action};
use vpt_application::Settings;
use vpt_application::ports::Clock;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::grounding::GroundedClaim;
use vpt_domain::identity::RecordingId;
use vpt_domain::proposal::{CheckedAction, CheckedClaim, Claim, Owner, action_ids};

const TEXT: &str = "vpt test: a made-up follow-up that checks vpt can send and withdraw one; \
                    vpt withdraws it at once, so remove it if it is still here";

const REACH: &str = "; the made-up follow-up went to whatever it calls";

pub(super) fn test(settings: &Settings) -> Result<Run<'_>, Row> {
    if settings.actions_command.is_empty() {
        return Err(Row::not_run(
            "command",
            "not configured, set `actions.command` in the config",
        ));
    }
    if settings.actions_withdraw_command.is_empty() {
        return Err(Row::not_run(
            "withdraw_command",
            "skipped, since without a withdraw command the test follow-up would stay behind; set \
             `actions.withdraw_command` in the config",
        ));
    }
    Ok(Box::new(|| rows(settings)))
}

fn rows(settings: &Settings) -> Vec<Row> {
    let Some(recording) = fresh() else {
        return vec![Row::issue(
            "send",
            "could not make a fresh id for the test follow-up",
        )];
    };
    let action = CheckedAction {
        claim: CheckedClaim {
            claim: Claim {
                text: TEXT.into(),
                source_references: Vec::new(),
            },
            grounding: GroundedClaim {
                sources: Vec::new(),
                finding: None,
                inherited_flags: Vec::new(),
            },
        },
        owner: Some(Owner::Me),
        due: None,
    };
    let id = action_ids(&recording, std::slice::from_ref(&action)).remove(0);
    let started = Instant::now();
    let sent = send_action(
        settings,
        &ActionSend {
            recording: &recording,
            action: &action,
            id: &id,
            note: "",
            link: "",
        },
    );
    let seconds = started.elapsed().as_secs_f64();
    let send = match sent {
        SendOutcome::Sent => Row::ok("send", format!("sent `{id}` in {seconds:.1} s{REACH}")),
        failed => Row::issue(
            "send",
            format!(
                "{} `{id}` in {seconds:.1} s{REACH}",
                if failed == SendOutcome::Refused {
                    "the command refused"
                } else {
                    "could not send"
                }
            ),
        )
        .advice("see `vpt doctor actions`, then run `actions.command` by hand"),
    };
    if sent != SendOutcome::Sent {
        return vec![
            send,
            Row::not_run("withdraw", "not run, since nothing was sent"),
        ];
    }
    let started = Instant::now();
    let withdrawn = withdraw_action(settings, &recording, &id);
    let seconds = started.elapsed().as_secs_f64();
    let withdraw = if withdrawn == WithdrawOutcome::Withdrawn {
        Row::ok("withdraw", format!("withdrew `{id}` in {seconds:.1} s"))
    } else {
        Row::issue(
            "withdraw",
            format!("could not withdraw `{id}` in {seconds:.1} s, so it is still there"),
        )
        .advice(format!(
            "remove the follow-up `{id}` by hand, then see `vpt doctor actions`"
        ))
    };
    vec![send, withdraw]
}

fn fresh() -> Option<RecordingId> {
    let now = SystemClock.now();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .subsec_nanos();
    let mut bytes = [0; 32];
    bytes[..4].copy_from_slice(&nanos.to_be_bytes());
    bytes[4..8].copy_from_slice(&std::process::id().to_be_bytes());
    RecordingId::derive(now, SystemClock.offset_at(now), &Sha256Digest(bytes)).ok()
}
