use crate::common::protocol::deserializer::DocumentDeserializer;
use crate::common::protocol::messages::response::Response;
use crate::common::protocol::messages::response::Status;

use crate::sync::protocol::v37::Protocol37 as P37Sync;

use crate::{OrientError, OrientResult};
use async_std::io::Read;
use async_std::net::TcpStream;
use async_trait::async_trait;

use super::reader;

use crate::common::protocol::messages::response::{
    Connect, CreateDB, DropDB, ExistDB, Header, Open, Query, QueryClose,
};
use crate::common::types::error::{OError, RequestError};
use crate::common::types::OResult;
use std::collections::{HashMap, VecDeque};

pub async fn decode(version: i16, buf: &mut TcpStream) -> OrientResult<Response> {
    if version >= 37 {
        return decode_with::<Protocol37>(buf).await;
    }
    Err(OrientError::Protocol(format!(
        "Protocol {} not supported",
        version
    )))
}

struct Protocol37 {}

#[async_trait]
impl VersionedDecoder for Protocol37 {
    async fn decode_header(buf: &mut TcpStream) -> OrientResult<Header> {
        let status = reader::read_i8(buf).await?;
        let session_id = reader::read_i32(buf).await?;

        let token = reader::read_optional_bytes(buf).await?;
        let op = reader::read_i8(buf).await?;

        Ok(Header {
            status: Status::from(status),
            client_id: None,
            session_id,
            token,
            op,
        })
    }
    async fn decode_open(buf: &mut TcpStream) -> OrientResult<Open> {
        let session_id = reader::read_i32(buf).await?;
        let token = reader::read_optional_bytes(buf).await?;
        Ok(Open::new(session_id, token))
    }

    async fn decode_errors(buf: &mut TcpStream) -> OrientResult<RequestError> {
        let code = reader::read_i32(buf).await?;
        let identifier = reader::read_i32(buf).await?;
        let mut errors = vec![];
        loop {
            let more = reader::read_bool(buf).await?;
            if more {
                let err_t = reader::read_string(buf).await?;
                let err_m = reader::read_string(buf).await?;
                let err = OError::new(err_t, err_m);
                errors.push(err);
            } else {
                break;
            }
        }
        let _serialized_exception = reader::read_bytes(buf).await?;
        Ok(RequestError {
            session_id: -1,
            code,
            identifier,
            errors,
            serialized: _serialized_exception,
        })
    }
    async fn decode_query(buf: &mut TcpStream) -> OrientResult<Query> {
        let query_id = reader::read_string(buf).await?;
        let changes = reader::read_bool(buf).await?;
        let has_plan = reader::read_bool(buf).await?;

        let execution_plan = if has_plan {
            Some(read_result(buf).await?)
        } else {
            None
        };

        let _prefetched = reader::read_i32(buf).await?;
        let records = read_result_set(buf).await?;
        let has_next = reader::read_bool(buf).await?;
        let stats = read_query_stats(buf).await?;
        let _reaload_metadata = reader::read_bool(buf).await?;

        Ok(Query::new(
            query_id,
            changes,
            execution_plan,
            records,
            has_next,
            stats,
        ))
    }
    async fn decode_connect(buf: &mut TcpStream) -> OrientResult<Connect> {
        let session_id = reader::read_i32(buf).await?;
        let token = reader::read_optional_bytes(buf).await?;
        Ok(Connect::new(session_id, token))
    }
    async fn decode_exist(buf: &mut TcpStream) -> OrientResult<ExistDB> {
        let exist = reader::read_bool(buf).await?;
        Ok(ExistDB::new(exist))
    }
}

#[async_trait]
pub trait VersionedDecoder {
    async fn decode_header(buf: &mut TcpStream) -> OrientResult<Header>;

    async fn decode_open(buf: &mut TcpStream) -> OrientResult<Open>;

    async fn decode_errors(buf: &mut TcpStream) -> OrientResult<RequestError>;

    async fn decode_query(buf: &mut TcpStream) -> OrientResult<Query>;

    async fn decode_connect(buf: &mut TcpStream) -> OrientResult<Connect>;

    async fn decode_exist(buf: &mut TcpStream) -> OrientResult<ExistDB>;

    async fn decode_drop_db(_buf: &mut TcpStream) -> OrientResult<DropDB> {
        Ok(DropDB {})
    }
    async fn decode_create_db(_buf: &mut TcpStream) -> OrientResult<CreateDB> {
        Ok(CreateDB {})
    }
    async fn decode_query_close<R: Read>(_buf: &mut R) -> OrientResult<QueryClose>
    where
        R: std::marker::Send,
    {
        Ok(QueryClose {})
    }
}

pub async fn decode_with<T: VersionedDecoder>(buf: &mut TcpStream) -> OrientResult<Response> {
    let header = T::decode_header(buf).await?;

    let payload = match header.status {
        Status::ERROR => return Err(OrientError::Request(T::decode_errors(buf).await?)),
        _ => match header.op {
            2 => T::decode_connect(buf).await?.into(),
            3 => T::decode_open(buf).await?.into(),
            4 => T::decode_create_db(buf).await?.into(),
            6 => T::decode_exist(buf).await?.into(),
            7 => T::decode_drop_db(buf).await?.into(),
            45 => T::decode_query(buf).await?.into(),
            46 => T::decode_query_close(buf).await?.into(),
            47 => T::decode_query(buf).await?.into(),
            _ => panic!("Request {} not supported", header.op),
        },
    };
    Ok(Response::new(header, payload))
}

async fn read_result(buf: &mut TcpStream) -> OrientResult<OResult> {
    let r_type = reader::read_i8(buf).await?;
    match r_type {
        4 => {
            let buffer = reader::read_bytes(buf).await?;
            let projection = P37Sync::decode_projection(&buffer)?;
            Ok(OResult::from(projection))
        }
        1 | 2 | 3 => {
            let _val = reader::read_i16(buf).await?;
            let _d_type = reader::read_i8(buf).await?;
            let identity = reader::read_identity(buf).await?;
            let version = reader::read_i32(buf).await?;

            let buffer = reader::read_bytes(buf).await?;
            let mut document = P37Sync::decode_document(&buffer)?;

            document.set_record_id(identity);
            document.set_version(version);

            Ok(OResult::from((r_type, document)))
        }
        _ => panic!("Unsupported result type {}", r_type),
    }
}

async fn read_result_set(buf: &mut TcpStream) -> OrientResult<VecDeque<OResult>> {
    let size = reader::read_i32(buf).await?;
    let mut records = VecDeque::new();
    for _ in 0..size {
        let result = read_result(buf).await?;
        records.push_back(result);
    }

    Ok(records)
}

async fn read_query_stats(buf: &mut TcpStream) -> OrientResult<HashMap<String, i64>> {
    let size = reader::read_i32(buf).await?;
    let stats = HashMap::new();
    for _ in 0..size {}
    Ok(stats)
}
