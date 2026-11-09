use crate::ledger::occasion::{
    codec,
    validate::{invalid, validate},
};
use rusqlite::{Connection, Row};
use vpt_application::ports::{LedgerError, OccasionRecord};
use vpt_domain::{
    occasion::{OccasionDetails, OccasionId, OccasionKey, time::OccasionTime},
    time::UtcInstant,
};

fn select(connection: &Connection) -> Result<String, LedgerError> {
    let at = if super::super::migrations::current(connection)? >= 12 {
        "at_rfc3339"
    } else {
        "NULL"
    };
    let mut columns = Vec::new();
    for (name, maximum) in [
        ("id", 30),
        ("provider_key", 524352),
        ("source", 6),
        ("title", 262144),
        ("participants", 8388608),
        ("tags", 8388608),
        ("brief_path", 65536),
        (at, 65536),
    ] {
        columns.push(format!(
            "CASE WHEN length(CAST({name} AS BLOB)) <= {maximum} THEN {name} END"
        ));
    }
    Ok(format!(
        "SELECT {}, at, duration_secs, briefed_at, brief_path IS NOT NULL FROM occasions",
        columns.join(",")
    ))
}

pub(in crate::ledger::sqlite) fn one(
    connection: &Connection,
    predicate: &str,
    value: &str,
) -> Result<Option<OccasionRecord>, LedgerError> {
    let sql = format!("{} WHERE {predicate}", select(connection)?);
    let mut statement = connection.prepare(&sql).map_err(super::super::map)?;
    let mut rows = statement.query([value]).map_err(super::super::map)?;
    rows.next()
        .map_err(super::super::map)?
        .map(decode)
        .transpose()
}

pub(super) fn all(connection: &Connection) -> Result<Vec<OccasionRecord>, LedgerError> {
    let mut statement = connection
        .prepare(&select(connection)?)
        .map_err(super::super::map)?;
    let mut rows = statement.query([]).map_err(super::super::map)?;
    let mut result = Vec::new();
    let mut budget = crate::ledger::occasion::list::Budget::default();
    while let Some(row) = rows.next().map_err(super::super::map)? {
        let record = decode(row)?;
        budget.admit(&record)?;
        result.push(record);
    }
    crate::ledger::occasion::list::sort(&mut result);
    Ok(result)
}

fn decode(row: &Row<'_>) -> Result<OccasionRecord, LedgerError> {
    let text = |index| {
        row.get_ref(index)
            .map_err(|_| invalid())?
            .as_str()
            .map_err(|_| invalid())
    };
    let id = OccasionId::parse(text(0)?).map_err(|_| invalid())?;
    let key = OccasionKey::parse(text(1)?).map_err(|_| invalid())?;
    if key.source().as_str() != text(2)? {
        return Err(invalid());
    }
    let at = OccasionTime::parse(text(7)?).map_err(|_| invalid())?;
    let seconds: i64 = row.get(8).map_err(|_| invalid())?;
    if at.unix_second().secs != seconds {
        return Err(invalid());
    }
    let duration: Option<i64> = row.get(9).map_err(|_| invalid())?;
    let briefed_at: Option<i64> = row.get(10).map_err(|_| invalid())?;
    let has_path: bool = row.get(11).map_err(|_| invalid())?;
    let record = OccasionRecord {
        id,
        key,
        details: OccasionDetails {
            at,
            title: text(3)?.into(),
            duration_secs: duration
                .map(u64::try_from)
                .transpose()
                .map_err(|_| invalid())?,
            participants: codec::read_participants(text(4)?)?,
            tags: codec::read_tags(text(5)?)?,
        },
        brief_path: if has_path {
            Some(text(6)?.into())
        } else {
            None
        },
        briefed_at: briefed_at.map(|secs| UtcInstant { secs }),
    };
    validate(&record)?;
    Ok(record)
}
