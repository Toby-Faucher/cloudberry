//! Saved shared playlists: anyone can create, rename, delete, edit and queue them.
//! In memory on `Station` (most recently updated first), written through to SQLite per change.

use crate::station::{QUEUE_FULL, Shared, Station, now_ms};
use axum::extract::ws::Utf8Bytes;
use rusqlite::{Connection, params};
use serde::Serialize;
use serde_json::json;
use std::collections::HashSet;
use std::hash::{BuildHasher, Hasher};
use std::sync::Arc;

pub const MAX_PLAYLISTS: usize = 100;
pub const MAX_TRACKS: usize = 500;
const MAX_NAME: usize = 60;
pub const BAD_NAME: &str = "Playlist name must be 1–60 characters";
pub const UNKNOWN: &str = "Unknown playlist";
pub const TOO_MANY: &str = "Too many playlists";
pub const FULL: &str = "Playlist is full";
pub const NO_SUCH_SONG: &str = "No such song in that playlist";
pub const EMPTY: &str = "That playlist is empty";
const UNKNOWN_TRACKS: &str = "Unknown track";
const SAVE_FAILED: &str = "Couldn't save the playlist";

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Playlist {
    pub id: u64,
    pub name: String,
    pub created_by: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub track_ids: Vec<String>,
    pub source_list_id: Option<String>,
}

pub fn playlists_msg(st: &Station) -> Utf8Bytes {
    json!({ "type": "playlists", "playlists": st.playlists })
        .to_string()
        .into()
}

fn clean_name(name: &str) -> Result<String, String> {
    let n = name.trim();
    if (1..=MAX_NAME).contains(&n.chars().count()) {
        Ok(n.to_string())
    } else {
        Err(BAD_NAME.into())
    }
}

/// Ids that are in the library, in the given order (unknown ones are skipped).
fn known(st: &Station, ids: Vec<String>) -> Vec<String> {
    ids.into_iter()
        .filter(|id| st.track(id).is_some())
        .collect()
}

fn songs(n: usize) -> String {
    format!("{n} song{}", if n == 1 { "" } else { "s" })
}

/// Fisher–Yates with a xorshift seeded from std's per-process random hasher keys.
fn shuffle<T>(v: &mut [T]) {
    let mut x = std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish()
        | 1;
    for i in (1..v.len()).rev() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        v.swap(i, (x % (i as u64 + 1)) as usize);
    }
}

/// Write one playlist and its tracks in a transaction; `id == 0` inserts and assigns the id.
fn save(db: &mut Connection, p: &mut Playlist) -> rusqlite::Result<()> {
    let tx = db.transaction()?;
    tx.execute(
        "INSERT INTO playlists (id, name, created_by, created_at, updated_at, source_list_id)
             VALUES (NULLIF(?1, 0), ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name,
                 updated_at = excluded.updated_at, source_list_id = excluded.source_list_id",
        params![
            p.id,
            p.name,
            p.created_by,
            p.created_at,
            p.updated_at,
            p.source_list_id
        ],
    )?;
    let id = if p.id == 0 {
        tx.last_insert_rowid() as u64
    } else {
        p.id
    };
    tx.execute("DELETE FROM playlist_tracks WHERE playlist_id = ?1", [id])?;
    {
        let mut ins = tx.prepare("INSERT INTO playlist_tracks VALUES (?1, ?2, ?3)")?;
        for (i, t) in p.track_ids.iter().enumerate() {
            ins.execute(params![id, i as i64, t])?;
        }
    }
    tx.commit()?;
    p.id = id;
    Ok(())
}

pub fn load(db: &Connection) -> rusqlite::Result<Vec<Playlist>> {
    let mut lists: Vec<Playlist> = db
        .prepare(
            "SELECT id, name, created_by, created_at, updated_at, source_list_id FROM playlists
                 ORDER BY updated_at DESC, id DESC",
        )?
        .query_map([], |r| {
            Ok(Playlist {
                id: r.get(0)?,
                name: r.get(1)?,
                created_by: r.get(2)?,
                created_at: r.get(3)?,
                updated_at: r.get(4)?,
                track_ids: Vec::new(),
                source_list_id: r.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let rows: Vec<(u64, String)> = db
        .prepare(
            "SELECT playlist_id, track_id FROM playlist_tracks ORDER BY playlist_id, position",
        )?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (id, t) in rows {
        if let Some(p) = lists.iter_mut().find(|p| p.id == id) {
            p.track_ids.push(t);
        }
    }
    Ok(lists)
}

impl Shared {
    /// Persist `p` (stamped as updated now), put it first, broadcast. Memory changes only if
    /// the write succeeded, so the two never disagree.
    fn store(&self, st: &mut Station, mut p: Playlist) -> Result<Playlist, String> {
        p.updated_at = now_ms() as u64;
        if let Err(e) = save(&mut self.db(), &mut p) {
            tracing::error!("saving playlist: {e}");
            return Err(SAVE_FAILED.into());
        }
        st.playlists.retain(|x| x.id != p.id);
        st.playlists.insert(0, p.clone());
        self.send(playlists_msg(st));
        Ok(p)
    }

    /// Edit a copy of playlist `id` with `f`, then store it.
    fn edit(
        &self,
        id: u64,
        f: impl FnOnce(&Station, &mut Playlist) -> Result<(), String>,
    ) -> Result<Playlist, String> {
        let mut st = self.lock();
        let mut p = st
            .playlists
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or(UNKNOWN)?;
        f(&st, &mut p)?;
        self.store(&mut st, p)
    }

    pub fn playlist_create(&self, name: &str, ids: Vec<String>, by: &str) -> Result<(), String> {
        let name = clean_name(name)?;
        let mut st = self.lock();
        if st.playlists.len() >= MAX_PLAYLISTS {
            return Err(TOO_MANY.into());
        }
        let track_ids = known(&st, ids);
        if track_ids.len() > MAX_TRACKS {
            return Err(FULL.into());
        }
        let now = now_ms() as u64;
        let p = Playlist {
            id: 0,
            name,
            created_by: by.to_string(),
            created_at: now,
            updated_at: now,
            track_ids,
            source_list_id: None,
        };
        let p = self.store(&mut st, p)?;
        drop(st);
        self.event(by, &format!("created playlist {}", p.name));
        Ok(())
    }

    pub fn playlist_rename(&self, id: u64, name: &str, by: &str) -> Result<(), String> {
        let name = clean_name(name)?;
        let mut old = String::new();
        self.edit(id, |_, p| {
            old = std::mem::replace(&mut p.name, name.clone());
            Ok(())
        })?;
        self.event(by, &format!("renamed playlist {old} to {name}"));
        Ok(())
    }

    pub fn playlist_delete(&self, id: u64, by: &str) -> Result<(), String> {
        let mut st = self.lock();
        let i = st
            .playlists
            .iter()
            .position(|p| p.id == id)
            .ok_or(UNKNOWN)?;
        let r = (|| {
            let mut db = self.db();
            let tx = db.transaction()?;
            tx.execute("DELETE FROM playlist_tracks WHERE playlist_id = ?1", [id])?;
            tx.execute("DELETE FROM playlists WHERE id = ?1", [id])?;
            tx.commit()
        })();
        if let Err(e) = r {
            tracing::error!("deleting playlist: {e}");
            return Err(SAVE_FAILED.into());
        }
        let p = st.playlists.remove(i);
        self.send(playlists_msg(&st));
        drop(st);
        self.event(by, &format!("deleted playlist {}", p.name));
        Ok(())
    }

    /// Append library tracks (unknown ids skipped); all or nothing against the 500 cap.
    pub fn playlist_add(&self, id: u64, ids: Vec<String>) -> Result<(), String> {
        self.edit(id, |st, p| {
            let add = known(st, ids);
            if add.is_empty() {
                return Err(UNKNOWN_TRACKS.into());
            }
            if p.track_ids.len() + add.len() > MAX_TRACKS {
                return Err(FULL.into());
            }
            p.track_ids.extend(add);
            Ok(())
        })
        .map(drop)
    }

    pub fn playlist_remove(&self, id: u64, index: usize) -> Result<(), String> {
        self.edit(id, |_, p| {
            if index >= p.track_ids.len() {
                return Err(NO_SUCH_SONG.into());
            }
            p.track_ids.remove(index);
            Ok(())
        })
        .map(drop)
    }

    /// Move the song at `from` to `to` (clamped to the end).
    pub fn playlist_move(&self, id: u64, from: usize, to: usize) -> Result<(), String> {
        self.edit(id, |_, p| {
            if from >= p.track_ids.len() {
                return Err(NO_SUCH_SONG.into());
            }
            let t = p.track_ids.remove(from);
            p.track_ids.insert(to.min(p.track_ids.len()), t);
            Ok(())
        })
        .map(drop)
    }

    /// Append the playlist to the queue (optionally shuffled) as far as the queue cap allows.
    /// Ok(Some(notice)) when only part of it fit.
    pub fn playlist_queue(
        self: &Arc<Self>,
        id: u64,
        shuffled: bool,
        by: &str,
    ) -> Result<Option<String>, String> {
        let (name, mut ids) = {
            let st = self.lock();
            let p = st.playlists.iter().find(|p| p.id == id).ok_or(UNKNOWN)?;
            (p.name.clone(), p.track_ids.clone())
        };
        if ids.is_empty() {
            return Err(EMPTY.into());
        }
        if shuffled {
            shuffle(&mut ids);
        }
        let mut queued = 0;
        self.change(|st, now| {
            for id in &ids {
                let Some(t) = st.track(id).cloned() else {
                    continue;
                };
                if !st.push(t, by, now) {
                    break;
                }
                queued += 1;
            }
            queued > 0
        });
        if queued == 0 {
            return Err(QUEUE_FULL.into());
        }
        let verb = if shuffled { "shuffled" } else { "queued" };
        self.event(by, &format!("{verb} playlist {name} ({})", songs(queued)));
        Ok(
            (queued < ids.len())
                .then(|| format!("Queued {queued} of {}, queue is full", ids.len())),
        )
    }

    /// A YouTube import: (re)write the playlist saved for `list_id` with the entries already in
    /// the library, in YouTube order; downloads slot in as they finish (`import_landed`).
    /// A re-import keeps the saved playlist's name (someone may have renamed it).
    pub fn save_import(&self, list_id: &str, pl: &crate::ytdl::Playlist, by: &str) {
        let mut st = self.lock();
        let ids = pl.entries.iter().map(|(id, _)| id.clone()).collect();
        let mut track_ids = known(&st, ids);
        track_ids.truncate(MAX_TRACKS);
        let existing = st
            .playlists
            .iter()
            .find(|p| p.source_list_id.as_deref() == Some(list_id))
            .cloned();
        let p = match existing {
            Some(p) => Playlist { track_ids, ..p },
            None if st.playlists.len() >= MAX_PLAYLISTS => {
                tracing::warn!(list_id, "not saving imported playlist: too many playlists");
                return;
            }
            None => {
                let name: String = pl.title.trim().chars().take(MAX_NAME).collect();
                let now = now_ms() as u64;
                Playlist {
                    id: 0,
                    name: if name.trim().is_empty() {
                        "YouTube playlist".into()
                    } else {
                        name.trim().to_string()
                    },
                    created_by: by.to_string(),
                    created_at: now,
                    updated_at: now,
                    track_ids,
                    source_list_id: Some(list_id.to_string()),
                }
            }
        };
        let _ = self.store(&mut st, p);
    }

    /// An import's download finished: insert it after the songs that precede it on YouTube.
    pub(crate) fn import_landed(&self, list_id: &str, before: &[String], track_id: &str) {
        let mut st = self.lock();
        let Some(mut p) = st
            .playlists
            .iter()
            .find(|p| p.source_list_id.as_deref() == Some(list_id))
            .cloned()
        else {
            return; // deleted meanwhile, or never saved (too many playlists)
        };
        if p.track_ids.len() >= MAX_TRACKS {
            return;
        }
        let before: HashSet<&str> = before.iter().map(String::as_str).collect();
        let at = p
            .track_ids
            .iter()
            .filter(|t| before.contains(t.as_str()))
            .count();
        p.track_ids.insert(at, track_id.to_string());
        let _ = self.store(&mut st, p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shuffle_is_a_permutation() {
        let mut v: Vec<u32> = (0..50).collect();
        shuffle(&mut v);
        let mut s = v.clone();
        s.sort();
        assert_eq!(s, (0..50).collect::<Vec<_>>());
        assert_ne!(
            v,
            (0..50).collect::<Vec<_>>(),
            "1 in 50! chance of a false failure"
        );
    }

    #[test]
    fn names_are_trimmed_and_bounded() {
        assert_eq!(clean_name("  Mix ").unwrap(), "Mix");
        assert_eq!(clean_name(&"é".repeat(60)).unwrap().chars().count(), 60);
        for bad in ["", "   ", &"x".repeat(61)] {
            assert_eq!(clean_name(bad).unwrap_err(), BAD_NAME);
        }
    }
}
