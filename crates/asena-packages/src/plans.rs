//! Plans handed out by `Plan` and consumed by `Commit`.
//!
//! A plan belongs to the D-Bus connection that asked for it and expires; a
//! commit recomputes it and proceeds only when APT would still make the same
//! changes, so nobody applies a transaction they were not shown.
use asena_core::packages::Change;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub const LIFETIME: Duration = Duration::from_secs(10 * 60);
const LIMIT: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stored {
    pub owner: String,
    pub install: Vec<String>,
    pub remove: Vec<String>,
    pub changes: Vec<Change>,
    pub created: Instant,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Lookup {
    Missing,
    OtherOwner,
    Expired,
}

#[derive(Default)]
pub struct Plans {
    entries: HashMap<String, Stored>,
}

impl Plans {
    pub fn prune(&mut self, now: Instant) {
        self.entries
            .retain(|_, stored| now.duration_since(stored.created) < LIFETIME);
    }

    pub fn insert(&mut self, id: String, plan: Stored, now: Instant) {
        self.prune(now);
        while self.entries.len() >= LIMIT {
            let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, stored)| stored.created)
                .map(|(id, _)| id.clone())
            else {
                break;
            };
            self.entries.remove(&oldest);
        }
        self.entries.insert(id, plan);
    }

    /// Removes and returns the plan when `owner` may commit it.
    pub fn take(&mut self, id: &str, owner: &str, now: Instant) -> Result<Stored, Lookup> {
        let stored = self.entries.get(id).ok_or(Lookup::Missing)?;
        if stored.owner != owner {
            // Another connection cannot learn more than that the plan is not theirs.
            return Err(Lookup::OtherOwner);
        }
        let stored = self.entries.remove(id).ok_or(Lookup::Missing)?;
        if now.duration_since(stored.created) >= LIFETIME {
            return Err(Lookup::Expired);
        }
        Ok(stored)
    }

    /// Drops the plans of a connection that left the bus.
    pub fn forget(&mut self, owner: &str) {
        self.entries.retain(|_, stored| stored.owner != owner);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asena_core::packages::Action;

    fn plan(owner: &str, created: Instant) -> Stored {
        Stored {
            owner: owner.into(),
            install: vec!["hello".into()],
            remove: Vec::new(),
            changes: vec![Change {
                name: "hello".into(),
                action: Action::Install,
                version: "2.10-5".into(),
                requested: true,
            }],
            created,
        }
    }

    #[test]
    fn only_the_owner_commits_and_only_once() {
        let now = Instant::now();
        let mut plans = Plans::default();
        plans.insert("a".into(), plan(":1.5", now), now);
        assert_eq!(plans.take("a", ":1.9", now), Err(Lookup::OtherOwner));
        assert_eq!(
            plans.take("a", ":1.5", now).map(|p| p.owner),
            Ok(":1.5".into())
        );
        assert_eq!(plans.take("a", ":1.5", now), Err(Lookup::Missing));
    }

    #[test]
    fn plans_expire() {
        let now = Instant::now();
        let mut plans = Plans::default();
        plans.insert("a".into(), plan(":1.5", now), now);
        let later = now + LIFETIME;
        assert_eq!(plans.take("a", ":1.5", later), Err(Lookup::Expired));
        plans.insert("b".into(), plan(":1.5", now), now);
        plans.prune(later);
        assert!(plans.is_empty());
    }

    #[test]
    fn the_store_stays_bounded_and_forgets_owners() {
        let now = Instant::now();
        let mut plans = Plans::default();
        for index in 0..(LIMIT + 5) {
            let created = now + Duration::from_millis(index as u64);
            plans.insert(format!("p{index}"), plan(":1.5", created), created);
        }
        assert_eq!(plans.entries.len(), LIMIT);
        assert!(!plans.entries.contains_key("p0"));
        plans.forget(":1.5");
        assert!(plans.is_empty());
    }
}
