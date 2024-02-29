use std::{collections::VecDeque, sync::Arc};

use serde_json::Value;

use crate::{
    ccc::CccHttpClient,
    model::{
        params::TunnelParams,
        proto::{LoginDisplayLabelSelect, ServerInfoResponse},
    },
};

pub async fn get(params: &TunnelParams) -> anyhow::Result<ServerInfoResponse> {
    let client = CccHttpClient::new(Arc::new(params.clone()), None);
    let info = client.get_server_info().await?;
    let response_data = info.get("ResponseData").unwrap_or(&Value::Null);
    Ok(serde_json::from_value::<ServerInfoResponse>(response_data.clone())?)
}

pub async fn get_pwd_prompts(params: &TunnelParams) -> anyhow::Result<VecDeque<String>> {
    let mut pwd_prompts = VecDeque::new();
    if !params.server_prompt {
        return Ok(pwd_prompts);
    }
    let server_info = get(params).await?;
    let login_type = &params.login_type;
    let login_factors = server_info
        .login_options_data
        .login_options_list
        .into_iter()
        .find_map(|login_option| {
            if login_option.id == *login_type {
                Some(login_option.factors)
            } else {
                None
            }
        })
        .unwrap_or_default();
    login_factors
        .into_iter()
        .filter_map(|factor| match &factor.custom_display_labels {
            LoginDisplayLabelSelect::LoginDisplayLabel(label) => Some(label.password.clone()),
            LoginDisplayLabelSelect::Empty(_) => None,
        })
        .for_each(|prompt| pwd_prompts.push_back(format!("{}: ", prompt.0.clone())));
    Ok(pwd_prompts)
}
