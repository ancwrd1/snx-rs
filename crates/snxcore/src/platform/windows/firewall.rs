use anyhow::anyhow;
use tracing::{debug, warn};
use windows::{
    Win32::{
        Foundation::{FWP_E_ALREADY_EXISTS, HANDLE},
        NetworkManagement::WindowsFilteringPlatform::{
            FWP_ACTION_BLOCK, FWP_ACTION_PERMIT, FWP_ACTION_TYPE, FWP_CONDITION_FLAG_IS_LOOPBACK, FWP_CONDITION_VALUE0,
            FWP_CONDITION_VALUE0_0, FWP_MATCH_FLAGS_ANY_SET, FWP_UINT8, FWP_UINT32, FWP_VALUE0, FWP_VALUE0_0,
            FWPM_ACTION0, FWPM_ACTION0_0, FWPM_CONDITION_FLAGS, FWPM_DISPLAY_DATA0, FWPM_FILTER_CONDITION0,
            FWPM_FILTER0, FWPM_LAYER_ALE_AUTH_CONNECT_V6, FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V6, FWPM_PROVIDER0,
            FWPM_SESSION_FLAG_DYNAMIC, FWPM_SESSION0, FWPM_SUBLAYER0, FwpmEngineClose0, FwpmEngineOpen0,
            FwpmFilterAdd0, FwpmProviderAdd0, FwpmSubLayerAdd0,
        },
        System::Rpc::RPC_C_AUTHN_DEFAULT,
    },
    core::{GUID, PWSTR},
};

use crate::utf16z;

// Random GUIDs unique for the application.
const PROVIDER_KEY: GUID = GUID::from_u128(0x93355376_a6f6_454d_b3f5_8f41e1de5178);
const SUBLAYER_KEY: GUID = GUID::from_u128(0x3e475762_4983_455e_9aaa_23a9b421ce1b);

const PROVIDER_NAME: &str = "snx-rs";
const SUBLAYER_NAME: &str = "snx-rs IPv6 block";

const WEIGHT_BLOCK: u8 = 10;
const WEIGHT_PERMIT_LOOPBACK: u8 = 15;

pub struct WfpIpv6Block {
    engine: HANDLE,
}

unsafe impl Send for WfpIpv6Block {}
unsafe impl Sync for WfpIpv6Block {}

impl WfpIpv6Block {
    fn open_dynamic_engine() -> anyhow::Result<HANDLE> {
        let session = FWPM_SESSION0 {
            flags: FWPM_SESSION_FLAG_DYNAMIC,
            ..Default::default()
        };

        let mut handle = HANDLE::default();
        let rc = unsafe { FwpmEngineOpen0(None, RPC_C_AUTHN_DEFAULT as u32, None, Some(&session), &mut handle) };
        if rc != 0 {
            return Err(anyhow!("FwpmEngineOpen0 failed: {rc:#x}"));
        }
        Ok(handle)
    }

    pub fn install() -> anyhow::Result<Self> {
        let this = Self {
            engine: Self::open_dynamic_engine()?,
        };

        this.add_provider()?;
        this.add_sublayer()?;

        this.add_loopback_permit(FWPM_LAYER_ALE_AUTH_CONNECT_V6)?;
        this.add_loopback_permit(FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V6)?;

        this.add_block(FWPM_LAYER_ALE_AUTH_CONNECT_V6)?;
        this.add_block(FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V6)?;

        debug!("Installed WFP IPv6 block (engine={:p})", this.engine.0);

        Ok(this)
    }

    fn add_provider(&self) -> anyhow::Result<()> {
        let mut name = utf16z!(PROVIDER_NAME);
        let provider = FWPM_PROVIDER0 {
            providerKey: PROVIDER_KEY,
            displayData: FWPM_DISPLAY_DATA0 {
                name: PWSTR(name.as_mut_ptr()),
                description: PWSTR::null(),
            },
            ..Default::default()
        };
        let rc = unsafe { FwpmProviderAdd0(self.engine, &provider, None) };
        if rc != 0 && rc != FWP_E_ALREADY_EXISTS.0 as u32 {
            return Err(anyhow!("FwpmProviderAdd0 failed: {rc:#x}"));
        }
        Ok(())
    }

    fn add_sublayer(&self) -> anyhow::Result<()> {
        let mut name = utf16z!(SUBLAYER_NAME);
        let mut provider_key = PROVIDER_KEY;
        let sublayer = FWPM_SUBLAYER0 {
            subLayerKey: SUBLAYER_KEY,
            displayData: FWPM_DISPLAY_DATA0 {
                name: PWSTR(name.as_mut_ptr()),
                description: PWSTR::null(),
            },
            providerKey: &mut provider_key,
            // High weight: sit above most third-party VPN/firewall sublayers.
            weight: 0x8000,
            ..Default::default()
        };
        let rc = unsafe { FwpmSubLayerAdd0(self.engine, &sublayer, None) };
        if rc != 0 && rc != FWP_E_ALREADY_EXISTS.0 as u32 {
            return Err(anyhow!("FwpmSubLayerAdd0 failed: {rc:#x}"));
        }
        Ok(())
    }

    fn add_block(&self, layer: GUID) -> anyhow::Result<()> {
        let mut name = utf16z!("snx-rs block IPv6");
        let filter = build_filter(&mut name, layer, FWP_ACTION_BLOCK, WEIGHT_BLOCK, &[]);
        self.add_filter(&filter)
    }

    fn add_loopback_permit(&self, layer: GUID) -> anyhow::Result<()> {
        let mut name = utf16z!("snx-rs permit IPv6 loopback");
        let condition = FWPM_FILTER_CONDITION0 {
            fieldKey: FWPM_CONDITION_FLAGS,
            matchType: FWP_MATCH_FLAGS_ANY_SET,
            conditionValue: FWP_CONDITION_VALUE0 {
                r#type: FWP_UINT32,
                Anonymous: FWP_CONDITION_VALUE0_0 {
                    uint32: FWP_CONDITION_FLAG_IS_LOOPBACK,
                },
            },
        };
        let conditions = [condition];
        let filter = build_filter(&mut name, layer, FWP_ACTION_PERMIT, WEIGHT_PERMIT_LOOPBACK, &conditions);
        self.add_filter(&filter)
    }

    fn add_filter(&self, filter: &FWPM_FILTER0) -> anyhow::Result<()> {
        let rc = unsafe { FwpmFilterAdd0(self.engine, filter, None, None) };
        if rc != 0 {
            return Err(anyhow!("FwpmFilterAdd0 failed: {rc:#x}"));
        }
        Ok(())
    }
}

impl Drop for WfpIpv6Block {
    fn drop(&mut self) {
        let rc = unsafe { FwpmEngineClose0(self.engine) };
        if rc != 0 {
            warn!("FwpmEngineClose0 failed: {rc:#x}");
        } else {
            debug!("Uninstalled WFP IPv6 block");
        }
    }
}

fn build_filter(
    name: &mut [u16],
    layer: GUID,
    action_type: FWP_ACTION_TYPE,
    weight: u8,
    conditions: &[FWPM_FILTER_CONDITION0],
) -> FWPM_FILTER0 {
    FWPM_FILTER0 {
        displayData: FWPM_DISPLAY_DATA0 {
            name: PWSTR(name.as_mut_ptr()),
            description: PWSTR::null(),
        },
        layerKey: layer,
        subLayerKey: SUBLAYER_KEY,
        weight: FWP_VALUE0 {
            r#type: FWP_UINT8,
            Anonymous: FWP_VALUE0_0 { uint8: weight },
        },
        action: FWPM_ACTION0 {
            r#type: action_type,
            Anonymous: FWPM_ACTION0_0 {
                filterType: GUID::zeroed(),
            },
        },
        numFilterConditions: conditions.len() as u32,
        filterCondition: if conditions.is_empty() {
            std::ptr::null_mut()
        } else {
            conditions.as_ptr() as *mut _
        },
        ..Default::default()
    }
}
