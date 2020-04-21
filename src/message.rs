use uuid::Uuid;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use crate::database::{Pool, extract_connection};
use crate::schema::{messages, devices, mailbox};
use crate::utils::{HandlerError, InternalError};
use chrono::Utc;

#[derive(Deserialize, Insertable)]
#[table_name = "messages"]
pub struct NewMessage {
    recipient: Uuid,
    #[serde(rename="type")]
    message_type: String,
    #[serde(skip)]
    pub sender: Uuid,

    payload: serde_json::Value
}

pub fn add_message(pool: &Pool, msg: NewMessage) -> Result<Uuid, HandlerError> {
    let conn = extract_connection(pool)?;

    conn.transaction::<Uuid, _, _>( || {

        let device_ids: Vec<Uuid> = devices::table.filter(devices::user_id.eq(msg.recipient))
            .select(devices::id).load::<Uuid>(&conn)?;

        let message_id = diesel::insert_into(messages::table).values(&msg)
            .returning(messages::id)
            .get_result::<Uuid>(&conn)?;

        let mbox_messages: Vec<_> = device_ids.iter().map(|x| (mailbox::device_id.eq(x), mailbox::message_id.eq(message_id))).collect();

        diesel::insert_into(mailbox::table)
            .values(&mbox_messages)
            .execute(&conn)?;
        Ok(message_id)
    }).map_err(|e| InternalError::DatabaseError(e).into())
}

#[derive(Serialize, Queryable)]
pub struct MailboxReturn {
    sender: Uuid,
    #[serde(rename="type")]
    message_type: String,
    timestamp: chrono::DateTime<Utc>,
    payload: serde_json::Value
}

pub fn check_mailbox(pool: &Pool, device_id: Uuid) -> Result<Vec<MailboxReturn>, HandlerError> {
    let conn = extract_connection(pool)?;

    conn.transaction::<Vec<MailboxReturn>, _, _>( || {

        let message_ids = diesel::delete(mailbox::table.filter(
            mailbox::device_id.eq(device_id)))
            .returning(mailbox::message_id)
            .load::<Uuid>(&conn)?;

        messages::table.filter(messages::id.eq_any(&message_ids))
            .select((messages::sender, messages::message_type, messages::reception_time, messages::payload))
            .load::<MailboxReturn>(&conn)
    }).map_err(|e| InternalError::DatabaseError(e).into())
}