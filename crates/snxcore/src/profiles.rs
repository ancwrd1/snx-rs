use std::{
    path::Path,
    sync::{Arc, LazyLock, Mutex, MutexGuard},
};

use uuid::Uuid;

use crate::model::params::{DEFAULT_PROFILE_UUID, TunnelParams};

static STORE: LazyLock<ConnectionProfilesStore> = LazyLock::new(ConnectionProfilesStore::new);

struct ProfilesState {
    profiles: Vec<Arc<TunnelParams>>,
    connected: Uuid,
}

impl ProfilesState {
    fn find(&self, uuid: Uuid) -> Option<Arc<TunnelParams>> {
        self.profiles.iter().find(|p| p.profile_id == uuid).cloned()
    }

    fn order(&self) -> Vec<Uuid> {
        self.profiles.iter().map(|p| p.profile_id).collect()
    }
}

pub struct ConnectionProfilesStore {
    state: Mutex<ProfilesState>,
}

impl ConnectionProfilesStore {
    fn new() -> Self {
        Self::new_in(TunnelParams::default_config_dir())
    }

    pub fn new_in<P: AsRef<Path>>(path: P) -> Self {
        let mut all = TunnelParams::load_all_from(path);
        if all.is_empty() {
            all.push(TunnelParams::default());
        }
        Self {
            state: Mutex::new(ProfilesState {
                profiles: all.into_iter().map(Arc::new).collect(),
                connected: DEFAULT_PROFILE_UUID,
            }),
        }
    }

    pub fn instance() -> &'static Self {
        &STORE
    }

    fn lock(&self) -> MutexGuard<'_, ProfilesState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn all(&self) -> Vec<Arc<TunnelParams>> {
        self.lock().profiles.clone()
    }

    pub fn reorder(&self, from: usize, to: usize) {
        let order = {
            let mut state = self.lock();
            if from >= state.profiles.len() || to >= state.profiles.len() || from == to {
                return;
            }
            let item = state.profiles.remove(from);
            state.profiles.insert(to, item);
            state.order()
        };
        let _ = TunnelParams::save_profile_order(&order);
    }

    pub fn get(&self, uuid: Uuid) -> Option<Arc<TunnelParams>> {
        self.lock().find(uuid)
    }

    /// Look up a profile by its UUID (string form) or by display name.
    pub fn find_by_name_or_uuid(&self, name_or_uuid: &str) -> Option<Arc<TunnelParams>> {
        let by_uuid = name_or_uuid.parse::<Uuid>().ok();
        self.lock()
            .profiles
            .iter()
            .find(|p| Some(p.profile_id) == by_uuid || p.profile_name == name_or_uuid)
            .cloned()
    }

    pub fn get_connected(&self) -> Arc<TunnelParams> {
        let state = self.lock();
        state
            .find(state.connected)
            .or_else(|| state.find(DEFAULT_PROFILE_UUID))
            .unwrap_or_else(|| Arc::new(TunnelParams::default()))
    }

    pub fn get_default(&self) -> Arc<TunnelParams> {
        self.get(DEFAULT_PROFILE_UUID)
            .unwrap_or_else(|| Arc::new(TunnelParams::default()))
    }

    pub fn set_connected(&self, uuid: Uuid) {
        self.lock().connected = uuid;
    }

    pub fn save(&self, params: Arc<TunnelParams>) {
        let _ = params.save();
        let order = {
            let mut state = self.lock();
            let mut added = false;
            if let Some(item) = state.profiles.iter_mut().find(|p| p.profile_id == params.profile_id) {
                *item = params.clone();
            } else {
                state.profiles.push(params.clone());
                added = true;
            }
            added.then(|| state.order())
        };
        if let Some(order) = order {
            let _ = TunnelParams::save_profile_order(&order);
        }
    }

    pub fn remove(&self, uuid: Uuid) {
        let (file, order) = {
            let mut state = self.lock();
            let file = state
                .profiles
                .iter()
                .find(|p| p.profile_id == uuid)
                .map(|p| p.config_file.clone());
            state.profiles.retain(|p| p.profile_id != uuid);
            (file, state.order())
        };
        if let Some(path) = file {
            let _ = std::fs::remove_file(path);
        }
        let _ = TunnelParams::save_profile_order(&order);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::TempDir;

    use super::*;

    fn write_profile(dir: &Path, name: &str, id: Uuid) -> Arc<TunnelParams> {
        let params = TunnelParams {
            profile_name: name.to_owned(),
            profile_id: id,
            config_file: dir.join(format!("{id}.conf")),
            ..Default::default()
        };
        params.save().unwrap();
        Arc::new(params)
    }

    #[test]
    fn new_in_empty_dir_creates_default_profile() {
        let dir = TempDir::new().unwrap();
        let store = ConnectionProfilesStore::new_in(dir.path());

        let all = store.all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].profile_id, DEFAULT_PROFILE_UUID);
    }

    #[test]
    fn new_in_loads_existing_profiles() {
        let dir = TempDir::new().unwrap();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        write_profile(dir.path(), "alpha", id1);
        write_profile(dir.path(), "beta", id2);

        let store = ConnectionProfilesStore::new_in(dir.path());
        let all = store.all();
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|p| p.profile_id == id1));
        assert!(all.iter().any(|p| p.profile_id == id2));
    }

    #[test]
    fn get_returns_profile_by_uuid() {
        let dir = TempDir::new().unwrap();
        let id = Uuid::new_v4();
        write_profile(dir.path(), "alpha", id);

        let store = ConnectionProfilesStore::new_in(dir.path());
        assert_eq!(store.get(id).unwrap().profile_id, id);
        assert!(store.get(Uuid::new_v4()).is_none());
    }

    #[test]
    fn find_by_name_or_uuid_matches_either() {
        let dir = TempDir::new().unwrap();
        let id = Uuid::new_v4();
        write_profile(dir.path(), "alpha", id);

        let store = ConnectionProfilesStore::new_in(dir.path());
        assert_eq!(store.find_by_name_or_uuid("alpha").unwrap().profile_id, id);
        assert_eq!(store.find_by_name_or_uuid(&id.to_string()).unwrap().profile_id, id);
        assert!(store.find_by_name_or_uuid("missing").is_none());
    }

    #[test]
    fn get_default_returns_default_profile() {
        let dir = TempDir::new().unwrap();
        let store = ConnectionProfilesStore::new_in(dir.path());
        assert_eq!(store.get_default().profile_id, DEFAULT_PROFILE_UUID);
    }

    #[test]
    fn set_connected_changes_get_connected() {
        let dir = TempDir::new().unwrap();
        let id = Uuid::new_v4();
        write_profile(dir.path(), "alpha", id);

        let store = ConnectionProfilesStore::new_in(dir.path());
        assert_eq!(store.get_connected().profile_id, DEFAULT_PROFILE_UUID);

        store.set_connected(id);
        assert_eq!(store.get_connected().profile_id, id);

        store.set_connected(Uuid::new_v4());
        assert_eq!(store.get_connected().profile_id, DEFAULT_PROFILE_UUID);
    }

    #[test]
    fn save_adds_and_updates_profile() {
        let dir = TempDir::new().unwrap();
        let store = ConnectionProfilesStore::new_in(dir.path());

        let id = Uuid::new_v4();
        let params = Arc::new(TunnelParams {
            profile_name: "alpha".to_owned(),
            profile_id: id,
            config_file: dir.path().join(format!("{id}.conf")),
            ..Default::default()
        });
        store.save(params.clone());
        assert_eq!(store.all().len(), 2);
        assert_eq!(store.get(id).unwrap().profile_name, "alpha");

        let updated = Arc::new(TunnelParams {
            profile_name: "renamed".to_owned(),
            ..(*params).clone()
        });
        store.save(updated);
        assert_eq!(store.all().len(), 2);
        assert_eq!(store.get(id).unwrap().profile_name, "renamed");
    }

    #[test]
    fn remove_drops_profile_and_deletes_file() {
        let dir = TempDir::new().unwrap();
        let id = Uuid::new_v4();
        let params = write_profile(dir.path(), "alpha", id);
        assert!(params.config_file.exists());

        let store = ConnectionProfilesStore::new_in(dir.path());
        store.remove(id);

        assert!(store.get(id).is_none());
        assert!(!params.config_file.exists());
    }

    #[test]
    fn reorder_swaps_profiles_in_memory() {
        let dir = TempDir::new().unwrap();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        write_profile(dir.path(), "alpha", id1);
        write_profile(dir.path(), "beta", id2);

        let store = ConnectionProfilesStore::new_in(dir.path());
        let before = store.all();
        let first = before[0].profile_id;
        let second = before[1].profile_id;

        store.reorder(0, 1);
        let after = store.all();
        assert_eq!(after[0].profile_id, second);
        assert_eq!(after[1].profile_id, first);

        store.reorder(5, 0);
        store.reorder(0, 0);
        let after = store.all();
        assert_eq!(after[0].profile_id, second);
        assert_eq!(after[1].profile_id, first);
    }
}
