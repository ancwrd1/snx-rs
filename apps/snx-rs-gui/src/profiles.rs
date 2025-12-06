use std::sync::{Arc, LazyLock, RwLock};

use snxcore::model::params::{DEFAULT_PROFILE_UUID, TunnelParams};
use uuid::Uuid;

static CONNECTION_PROFILES_STORE: LazyLock<ConnectionProfilesStore> = LazyLock::new(ConnectionProfilesStore::new);

pub struct ConnectionProfilesStore {
    profiles: RwLock<Vec<Arc<TunnelParams>>>,
    connected_profile: RwLock<Uuid>,
}

impl ConnectionProfilesStore {
    fn new() -> Self {
        let mut all = TunnelParams::load_all();
        if all.is_empty() {
            all.push(TunnelParams::default());
        }
        Self {
            profiles: RwLock::new(all.into_iter().map(Arc::new).collect()),
            connected_profile: RwLock::new(DEFAULT_PROFILE_UUID),
        }
    }

    pub fn instance() -> &'static Self {
        &CONNECTION_PROFILES_STORE
    }

    pub fn all(&self) -> Vec<Arc<TunnelParams>> {
        self.profiles.read().unwrap().clone()
    }

    pub fn get(&self, uuid: Uuid) -> Option<Arc<TunnelParams>> {
        self.profiles
            .read()
            .unwrap()
            .iter()
            .find(|p| p.profile_id == uuid)
            .cloned()
    }

    pub fn get_connected(&self) -> Arc<TunnelParams> {
        self.get(*self.connected_profile.read().unwrap())
            .unwrap_or_else(|| Arc::new(TunnelParams::default()))
    }

    pub fn get_default(&self) -> Arc<TunnelParams> {
        self.get(DEFAULT_PROFILE_UUID)
            .unwrap_or_else(|| Arc::new(TunnelParams::default()))
    }

    pub fn set_connected(&self, uuid: Uuid) {
        *self.connected_profile.write().unwrap() = uuid;
    }

    pub fn save(&self, params: Arc<TunnelParams>) {
        let mut profiles = self.profiles.write().unwrap();
        if let Some(item) = profiles.iter_mut().find(|p| p.profile_id == params.profile_id) {
            *item = params.clone();
        } else {
            profiles.push(params.clone());
        }
        let _ = params.save();
    }

    pub fn remove(&self, uuid: Uuid) {
        let mut profiles = self.profiles.write().unwrap();
        if let Some(item) = profiles.iter_mut().find(|p| p.profile_id == uuid) {
            let _ = std::fs::remove_file(&item.config_file);
        }
        profiles.retain(|p| p.profile_id != uuid);
    }
}
