use crate::{ErrorPayload, Id, Request, Response, ResponsePayload, RpcObject};
use axum::extract;

impl From<extract::rejection::JsonRejection> for Response<(), ()> {
    fn from(value: extract::rejection::JsonRejection) -> Self {
        Self {
            id: Id::None,
            payload: ResponsePayload::Failure(ErrorPayload {
                code: -32600,
                message: value.to_string().into(),
                data: None,
            }),
        }
    }
}

impl<Payload, ErrData> axum::response::IntoResponse for Response<Payload, ErrData>
where
    Payload: RpcObject,
    ErrData: RpcObject,
{
    fn into_response(self) -> axum::response::Response {
        axum::response::IntoResponse::into_response(axum::response::Json(self))
    }
}

#[async_trait::async_trait]
impl<S, Params> extract::FromRequest<S> for Request<Params>
where
    axum::body::Bytes: extract::FromRequest<S>,
    Params: RpcObject,
    S: Send + Sync,
{
    type Rejection = Response<(), ()>;

    async fn from_request(req: extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        let json = extract::Json::<Self>::from_request(req, state).await?;

        Ok(json.0)
    }
}
