use std::{collections::VecDeque, sync::Arc};

use crate::{
    ccc::CccHttpClient,
    model::{
        params::TunnelParams,
        proto::{LoginDisplayLabelSelect, ServerInfoResponse},
    },
    sexpr2::SExpression,
};

pub async fn get(params: &TunnelParams) -> anyhow::Result<ServerInfoResponse> {
    let client = CccHttpClient::new(Arc::new(params.clone()), None);
    let info = client.get_server_info().await?;
    info.get("CCCserverResponse:ResponseData")
        .cloned()
        .unwrap_or(SExpression::Null)
        .try_into()
}

pub async fn get_mfa_prompts(params: &TunnelParams) -> anyhow::Result<VecDeque<String>> {
    let mut mfa_prompts = VecDeque::new();
    if !params.server_prompt {
        return Ok(mfa_prompts);
    }
    let server_info = get(params).await?;
    let login_type = &params.login_type;
    let login_factors = server_info
        .login_options_data
        .login_options_list
        .into_iter()
        .find_map(|login_option| {
            if login_option.1.id == *login_type {
                Some(login_option.1.factors)
            } else {
                None
            }
        })
        .unwrap_or_default();
    login_factors
        .into_iter()
        .filter_map(|factor| match &factor.1.custom_display_labels {
            LoginDisplayLabelSelect::LoginDisplayLabel(label) => label.password.clone(),
            _ => None,
        })
        .for_each(|prompt| mfa_prompts.push_back(format!("{}: ", prompt.0.clone())));

    Ok(mfa_prompts)
}
