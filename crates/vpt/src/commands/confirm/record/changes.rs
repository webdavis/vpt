use super::super::{Request, relation, tag};
use vpt_adapters::{FilesystemStores, Roots, SqliteLedger};
use vpt_application::{Settings, ports::*};
use vpt_domain::{tags::Action, time::UtcInstant};
use vpt_protocol::error::ErrorDocument;

pub(super) struct Prepared {
    pub delta: WorkDelta,
    pub confirmed: serde_json::Value,
    pub diagnostics: usize,
    pub message: &'static str,
}

pub(super) fn prepare(
    request: &Request<'_>,
    inputs: NoteInputs,
    settings: &Settings,
    roots: &Roots,
    stores: &FilesystemStores,
    ledger: &SqliteLedger,
    at: UtcInstant,
) -> Result<Prepared, Box<ErrorDocument>> {
    match request {
        Request::Tag(request) => {
            let (delta, diagnostics) =
                tag::changes::prepare(request, inputs, settings, roots, stores, at)?;
            let rejected = request.action == Action::Reject;
            Ok(Prepared {
                confirmed: serde_json::json!({"tag": !rejected, "rejected": rejected, "changed": delta.tags.is_some(), "added": delta.vocabulary.is_some()}),
                delta,
                diagnostics,
                message: if rejected {
                    "Rejected tag"
                } else {
                    "Confirmed tag"
                },
            })
        }
        Request::Relation(request) => {
            let (delta, changed, diagnostics) =
                relation::changes::prepare(request, inputs, settings, roots, stores, ledger, at)?;
            Ok(Prepared {
                delta,
                diagnostics,
                confirmed: serde_json::json!({"relation": true, "kind": request.kind.as_str(), "changed": changed}),
                message: "Confirmed relation",
            })
        }
        Request::Term(_) => Err(super::super::usage("recording is required")),
    }
}
