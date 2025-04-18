use std::{collections::VecDeque, sync::Arc};

use crate::{
    ccc::CccHttpClient,
    model::{
        params::TunnelParams,
        proto::{LoginDisplayLabelSelect, LoginOption, ServerInfoResponse},
        PromptInfo,
    },
    sexpr::SExpression,
};
use cached::proc_macro::cached;
use tracing::trace;

#[cached(
    result = true,
    ty = "cached::UnboundCache<String, ServerInfoResponse>",
    create = "{ cached::UnboundCache::new() }",
    convert = r#"{ params.server_name.clone() }"#
)]
pub async fn get(params: &TunnelParams) -> anyhow::Result<ServerInfoResponse> {
    let client = CccHttpClient::new(Arc::new(params.clone()), None);

    let info = client.get_server_info().await?;

    info.get("CCCserverResponse:ResponseData")
        .cloned()
        .unwrap_or(SExpression::Null)
        .try_into()
}

pub async fn get_login_prompts(params: &TunnelParams) -> anyhow::Result<VecDeque<PromptInfo>> {
    let factors = get_login_option(params)
        .await?
        .map(|o| o.factors)
        .unwrap_or_default()
        .into_values();

    let result = factors
        .filter_map(|factor| match factor.custom_display_labels {
            LoginDisplayLabelSelect::LoginDisplayLabel(map) => map.get("password").map(|label| {
                PromptInfo::new(
                    map.get("header").map(ToOwned::to_owned).unwrap_or_default(),
                    format!("{}: ", label),
                )
            }),
            LoginDisplayLabelSelect::Empty(_) => None,
        })
        .collect();

    trace!("Retrieved server prompts: {:?}", result);

    Ok(result)
}

pub async fn get_login_option(params: &TunnelParams) -> anyhow::Result<Option<LoginOption>> {
    let info = get(params).await?;

    let result = info.login_options_data.and_then(|data| {
        data.login_options_list
            .into_values()
            .find(|option| option.id == params.login_type)
    });

    Ok(result)
}
