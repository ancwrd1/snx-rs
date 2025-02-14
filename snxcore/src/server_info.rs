use crate::model::proto::LoginDisplayLabelSelect;
use crate::model::AuthPrompt;
use crate::{
    ccc::CccHttpClient,
    model::{
        params::TunnelParams,
        proto::{LoginFactor, ServerInfoResponse},
    },
    sexpr::SExpression,
};
use cached::proc_macro::cached;
use std::{collections::VecDeque, sync::Arc};
use tracing::trace;

pub async fn get(params: &TunnelParams) -> anyhow::Result<ServerInfoResponse> {
    let client = CccHttpClient::new(Arc::new(params.clone()), None);

    let info = client.get_server_info().await?;

    info.get("CCCserverResponse:ResponseData")
        .cloned()
        .unwrap_or(SExpression::Null)
        .try_into()
}

pub async fn get_mfa_prompts(params: &TunnelParams) -> anyhow::Result<VecDeque<AuthPrompt>> {
    let factors = get_login_factors(params).await?;

    let result = factors
        .into_iter()
        .filter_map(|factor| match factor.custom_display_labels {
            LoginDisplayLabelSelect::LoginDisplayLabel(map) => map.get("password").map(|label| {
                AuthPrompt::new(
                    map.get("header").map(ToOwned::to_owned).unwrap_or_default(),
                    factor.factor_type,
                    format!("{}: ", label),
                )
            }),
            LoginDisplayLabelSelect::Empty(_) => None,
        })
        .collect();

    trace!("Retrieved server prompts: {:?}", result);

    Ok(result)
}

#[cached(
    result = true,
    ty = "cached::UnboundCache<String, Vec<LoginFactor>>",
    create = "{ cached::UnboundCache::new() }",
    convert = r#"{ format!("{}/{}", params.server_name, params.login_type) }"#
)]
pub async fn get_login_factors(params: &TunnelParams) -> anyhow::Result<Vec<LoginFactor>> {
    let info = get(params).await?;

    let result = info
        .login_options_data
        .and_then(|data| {
            data.login_options_list.into_values().find_map(|option| {
                if option.id == params.login_type {
                    Some(option.factors.into_values().collect::<Vec<_>>())
                } else {
                    None
                }
            })
        })
        .unwrap_or_default();

    Ok(result)
}
