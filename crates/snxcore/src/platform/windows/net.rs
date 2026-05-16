use std::{
    ffi::c_void,
    net::Ipv4Addr,
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::anyhow;
use ipnet::Ipv4Net;
use tracing::{debug, trace, warn};
use windows::{
    Win32::{
        Foundation::{ERROR_NOT_FOUND, HANDLE, NO_ERROR},
        NetworkManagement::{
            IpHelper::{
                ConvertInterfaceAliasToLuid, CreateUnicastIpAddressEntry, DeleteUnicastIpAddressEntry, GetBestRoute2,
                GetIpInterfaceEntry, GetNetworkConnectivityHint, InitializeUnicastIpAddressEntry, MIB_IPFORWARD_ROW2,
                MIB_IPINTERFACE_ROW, MIB_NOTIFICATION_TYPE, MIB_UNICASTIPADDRESS_ROW, NotifyRouteChange2,
                SetIpInterfaceEntry,
            },
            Ndis::{IF_MAX_STRING_SIZE, NET_LUID_LH},
        },
        Networking::WinSock::{
            AF_INET, AF_UNSPEC, NL_NETWORK_CONNECTIVITY_HINT, NL_NETWORK_CONNECTIVITY_LEVEL_HINT,
            NetworkConnectivityLevelHintConstrainedInternetAccess, NetworkConnectivityLevelHintInternetAccess,
            SOCKADDR_INET,
        },
    },
    core::PCWSTR,
};

use crate::platform::{DeviceConfig, NetworkInterface, StatsPoller};

static ONLINE_STATE: AtomicBool = AtomicBool::new(true);

#[derive(Default)]
pub struct WindowsNetworkInterface;

impl WindowsNetworkInterface {
    pub fn new() -> Self {
        Self
    }
}

fn to_wide_nul(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn luid_for_alias(alias: &str) -> anyhow::Result<NET_LUID_LH> {
    if alias.len() >= IF_MAX_STRING_SIZE as usize {
        return Err(anyhow!("interface alias too long: {alias}"));
    }
    let wide = to_wide_nul(alias);
    let mut luid = NET_LUID_LH::default();
    unsafe { ConvertInterfaceAliasToLuid(PCWSTR(wide.as_ptr()), &mut luid) }
        .ok()
        .map_err(|e| anyhow!("ConvertInterfaceAliasToLuid({alias}) failed: {e}"))?;
    Ok(luid)
}

fn sockaddr_ipv4(addr: Ipv4Addr) -> SOCKADDR_INET {
    let mut sa = SOCKADDR_INET::default();
    let ipv4 = unsafe { &mut sa.Ipv4 };
    ipv4.sin_family = AF_INET;
    ipv4.sin_addr.S_un.S_addr = u32::from_ne_bytes(addr.octets());
    sa
}

fn ipv4_from_sockaddr(sa: &SOCKADDR_INET) -> Option<Ipv4Addr> {
    let family = unsafe { sa.si_family };
    if family != AF_INET {
        return None;
    }
    let raw = unsafe { sa.Ipv4.sin_addr.S_un.S_addr };
    Some(Ipv4Addr::from(u32::to_ne_bytes(raw)))
}

fn unicast_row(luid: NET_LUID_LH, addr: Ipv4Net) -> MIB_UNICASTIPADDRESS_ROW {
    let mut row = MIB_UNICASTIPADDRESS_ROW::default();
    unsafe { InitializeUnicastIpAddressEntry(&mut row) };
    row.InterfaceLuid = luid;
    row.Address = sockaddr_ipv4(addr.addr());
    row.OnLinkPrefixLength = addr.prefix_len();
    row
}

fn recompute_online() {
    let mut hint = NL_NETWORK_CONNECTIVITY_HINT::default();
    let rc = unsafe { GetNetworkConnectivityHint(&mut hint) };
    if rc == NO_ERROR {
        let level: NL_NETWORK_CONNECTIVITY_LEVEL_HINT = hint.ConnectivityLevel;
        let online = level == NetworkConnectivityLevelHintInternetAccess
            || level == NetworkConnectivityLevelHintConstrainedInternetAccess;
        ONLINE_STATE.store(online, Ordering::SeqCst);
        trace!("Network connectivity level: {:?}, online={}", level.0, online);
    } else {
        warn!("GetNetworkConnectivityHint failed: {:?}", rc);
    }
}

unsafe extern "system" fn route_change_callback(
    _ctx: *const c_void,
    _row: *const MIB_IPFORWARD_ROW2,
    _kind: MIB_NOTIFICATION_TYPE,
) {
    recompute_online();
}

impl NetworkInterface for WindowsNetworkInterface {
    async fn start_network_state_monitoring(&self) -> anyhow::Result<()> {
        recompute_online();
        let mut handle = HANDLE::default();
        let rc = unsafe {
            NotifyRouteChange2(
                AF_UNSPEC,
                Some(route_change_callback),
                std::ptr::null(),
                false,
                &mut handle,
            )
        };
        if rc != NO_ERROR {
            return Err(anyhow!("NotifyRouteChange2 failed: {:?}", rc));
        }
        // The notification handle is kept implicitly for the daemon's lifetime; we
        // don't need `CancelMibChangeNotify2` because the subscription is global.
        let _ = handle;
        Ok(())
    }

    async fn get_default_ipv4(&self) -> anyhow::Result<Ipv4Addr> {
        let dest = sockaddr_ipv4(Ipv4Addr::UNSPECIFIED);
        let mut row = MIB_IPFORWARD_ROW2::default();
        let mut best_src = SOCKADDR_INET::default();

        let rc = unsafe { GetBestRoute2(None, 0, None, &dest, 0, &mut row, &mut best_src) };
        if rc != NO_ERROR {
            return Err(anyhow!("GetBestRoute2 failed: {:?}", rc));
        }
        ipv4_from_sockaddr(&best_src).ok_or_else(|| anyhow!(i18n::tr!("error-cannot-determine-ip")))
    }

    async fn delete_device(&self, device_name: &str) -> anyhow::Result<()> {
        // Wintun adapters disappear when the `AsyncDevice` handle drops; nothing to do
        // here for the adapter itself. Address rows attached to the adapter are removed
        // along with it. We log so a missing-name situation isn't silent.
        debug!("delete_device({device_name}) — no-op on Windows (wintun handle owns lifecycle)");
        Ok(())
    }

    async fn configure_device(&self, device_config: &DeviceConfig) -> anyhow::Result<()> {
        debug!("Configuring device: {:?}", device_config);
        let luid = luid_for_alias(&device_config.name)?;

        let mut iface = MIB_IPINTERFACE_ROW {
            Family: AF_INET,
            InterfaceLuid: luid,
            ..Default::default()
        };

        unsafe { GetIpInterfaceEntry(&mut iface) }
            .ok()
            .map_err(|e| anyhow!("GetIpInterfaceEntry failed: {e}"))?;

        iface.NlMtu = device_config.mtu as u32;
        iface.SitePrefixLength = 0;
        iface.ForwardingEnabled = device_config.allow_forwarding;

        unsafe { SetIpInterfaceEntry(&mut iface) }
            .ok()
            .map_err(|e| anyhow!("SetIpInterfaceEntry failed: {e}"))?;

        let row = unicast_row(luid, device_config.address);
        unsafe { CreateUnicastIpAddressEntry(&row) }
            .ok()
            .map_err(|e| anyhow!("CreateUnicastIpAddressEntry failed: {e}"))?;

        Ok(())
    }

    async fn replace_ip_address(
        &self,
        device_name: &str,
        old_address: Ipv4Net,
        new_address: Ipv4Net,
    ) -> anyhow::Result<()> {
        let luid = luid_for_alias(device_name)?;
        let new_row = unicast_row(luid, new_address);
        unsafe { CreateUnicastIpAddressEntry(&new_row) }
            .ok()
            .map_err(|e| anyhow!("CreateUnicastIpAddressEntry failed: {e}"))?;

        let old_row = unicast_row(luid, old_address);
        let rc = unsafe { DeleteUnicastIpAddressEntry(&old_row) };

        if rc != NO_ERROR && rc != ERROR_NOT_FOUND {
            return Err(anyhow!("DeleteUnicastIpAddressEntry failed: {:?}", rc));
        }
        Ok(())
    }

    fn new_stats_poller(
        &self,
        device_name: &str,
    ) -> impl Future<Output = anyhow::Result<impl StatsPoller + Send + Sync + 'static>> + Send {
        let device_name = device_name.to_owned();
        async move { super::stats::WindowsStatsPoller::new(&device_name) }
    }

    fn is_online(&self) -> bool {
        ONLINE_STATE.load(Ordering::SeqCst)
    }
}
