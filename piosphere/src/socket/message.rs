use macros::request;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[request(Self, Hello)]
pub struct Hello;

#[derive(Debug, Serialize, Deserialize)]
#[request(Vec<crate::db::Deployment>, Overview)]
pub struct Overview;

#[derive(Debug, Serialize, Deserialize)]
#[request(crate::deployment::Deployment, ViewDeployment)]
pub struct ViewDeployment(pub String);

#[macro_export]
macro_rules! handle {
    ($self:ident, $stream:ident, $msg:ident, $($tag:ident => $handler:path,)*) => {
        let PiosphereRequest { tag, message } = $msg;

        match tag {
            $(
                PiosphereTag::$tag => {
                    let message = bincode::deserialize(&message)?;
                    let response = <Self as Handler<$handler>>::handle($self, message).await?;
                    $stream.write(response).await?;
                }
            ),*
        }
    };
}
