use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use super::state::UiState;
use crate::{
    domain::{
        affinity::Affinity,
        ids::UserId,
        presence::{Availability, Presence},
        user::UserSummary,
    },
    util::fuzzy::{
        PreparedFuzzyCandidate, fuzzy_match_prepared, prepare_fuzzy_candidate, prepare_fuzzy_query,
    },
};

#[derive(Clone, Debug)]
struct Person {
    user: UserSummary,
    username: PreparedFuzzyCandidate,
    name: PreparedFuzzyCandidate,
    contact: bool,
}

/// Prepared once when the picker opens, using only already-loaded data.
/// Arc keeps store snapshots cheap. Contact changes invalidate this snapshot.
#[derive(Clone, Debug, Default)]
pub struct PeopleCorpus(Arc<Vec<Person>>);

impl PeopleCorpus {
    pub fn from_state(state: &UiState) -> Self {
        let mut users = HashMap::<String, bool>::new();
        let mut add = |username: &str, contact: bool| {
            let username = username.trim().to_ascii_lowercase();
            // DM titles can also contain unresolved assertions. They are not usernames.
            if !username.is_empty()
                && username
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_')
            {
                *users.entry(username).or_default() |= contact;
            }
        };
        for id in state.backend.user_profiles.keys() {
            add(&id.0, false);
        }
        for (id, affinity) in &state.backend.user_affinities {
            add(&id.0, *affinity == Affinity::Positive);
        }
        for dm in &state.workspace.direct_messages {
            for username in dm.title.split(',') {
                add(username, true);
            }
        }
        for roster in state.backend.conversation_members.values() {
            for id in &roster.members {
                add(&id.0, false);
            }
        }
        for roles in state.backend.team_roles.values() {
            for id in roles.keys() {
                add(&id.0, false);
            }
        }
        let profiles = state
            .backend
            .user_profiles
            .iter()
            .map(|(id, profile)| (id.0.to_ascii_lowercase(), profile))
            .collect::<HashMap<_, _>>();
        let affinities = state
            .backend
            .user_affinities
            .iter()
            .map(|(id, affinity)| (id.0.to_ascii_lowercase(), *affinity))
            .collect::<HashMap<_, _>>();
        Self(Arc::new(
            users
                .into_iter()
                .map(|(username, contact)| {
                    let profile = profiles.get(&username);
                    let display_name = profile
                        .map(|profile| profile.display_name.clone())
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or_else(|| username.clone());
                    Person {
                        username: prepare_fuzzy_candidate(&username),
                        name: prepare_fuzzy_candidate(&display_name),
                        contact,
                        user: UserSummary {
                            id: UserId::new(username.clone()),
                            display_name,
                            title: String::new(),
                            avatar_asset: profile.and_then(|profile| profile.avatar_asset.clone()),
                            presence: state
                                .backend
                                .user_presences
                                .get(&UserId::new(username.clone()))
                                .cloned()
                                .unwrap_or(Presence {
                                    availability: Availability::Unknown,
                                    status_text: None,
                                }),
                            affinity: affinities.get(&username).copied().unwrap_or_default(),
                        },
                    }
                })
                .collect(),
        ))
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<UserSummary> {
        let Some(prepared) = prepare_fuzzy_query(query) else {
            return Vec::new();
        };
        let normalized = query.trim().to_lowercase();
        let mut matches = self
            .0
            .iter()
            .filter_map(|person| {
                let score = [
                    fuzzy_match_prepared(&prepared, &person.username),
                    fuzzy_match_prepared(&prepared, &person.name),
                ]
                .into_iter()
                .flatten()
                .map(|result| result.score)
                .max()?;
                let username = &person.user.id.0;
                let name = person.user.display_name.to_lowercase();
                let match_rank = if username == &normalized || name == normalized {
                    2
                } else if username.starts_with(&normalized) || name.starts_with(&normalized) {
                    1
                } else {
                    0
                };
                Some((match_rank, person.contact, score, person))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| a.3.user.id.0.cmp(&b.3.user.id.0))
        });
        matches
            .into_iter()
            .take(limit)
            .map(|(_, _, _, person)| person.user.clone())
            .collect()
    }
}

/// Append wider results without moving local rows or duplicating usernames.
pub fn merge_results(local: &mut Vec<UserSummary>, remote: Vec<UserSummary>) {
    let mut seen = local
        .iter()
        .map(|user| user.id.0.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    local.extend(
        remote
            .into_iter()
            .filter(|user| seen.insert(user.id.0.to_ascii_lowercase())),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::ConversationId;
    use crate::state::state::{ConversationMembersState, UserProfileState};

    pub(super) fn known_people() -> UiState {
        let mut state = UiState::default();
        for (username, name) in [
            ("alice", "Alice Example"),
            ("alicia", "Alicia Jones"),
            ("sam", "Samuel Rivera"),
        ] {
            state.backend.user_profiles.insert(
                UserId::new(username),
                UserProfileState {
                    display_name: name.into(),
                    avatar_asset: None,
                    updated_ms: 0,
                },
            );
        }
        state
            .backend
            .user_affinities
            .insert(UserId::new("alicia"), Affinity::Positive);
        state
            .backend
            .user_affinities
            .insert(UserId::new("followee"), Affinity::Positive);
        state.backend.conversation_members.insert(
            ConversationId::new("team"),
            ConversationMembersState {
                members: vec![UserId::new("teammate"), UserId::new("ALICE")],
                ..Default::default()
            },
        );
        state
    }

    #[test]
    fn people_search_uses_names_following_and_loaded_members() {
        let corpus = PeopleCorpus::from_state(&known_people());
        assert_eq!(corpus.search("rivera", 8)[0].id.0, "sam");
        assert_eq!(corpus.search("follow", 8)[0].id.0, "followee");
        assert_eq!(corpus.search("teamm", 8)[0].id.0, "teammate");
        assert_eq!(
            corpus
                .search("alice", 8)
                .iter()
                .filter(|user| user.id.0 == "alice")
                .count(),
            1
        );
        assert!(corpus.search("  ", 8).is_empty());
        assert!(corpus.search("unfindable", 8).is_empty());
    }

    #[test]
    fn people_search_prefers_exact_then_contact_prefix_matches_and_limits_results() {
        let corpus = PeopleCorpus::from_state(&known_people());
        assert_eq!(corpus.search("ali", 1)[0].id.0, "alicia");
        assert_eq!(corpus.search("alice", 1)[0].id.0, "alice");
        assert_eq!(corpus.search("ali", 1).len(), 1);
    }

    #[test]
    fn merging_preserves_local_order_and_deduplicates_both_sources() {
        let corpus = PeopleCorpus::from_state(&known_people());
        let mut local = corpus.search("ali", 2);
        let ids = local.iter().map(|user| user.id.clone()).collect::<Vec<_>>();
        let mut duplicate = local[0].clone();
        duplicate.id.0 = duplicate.id.0.to_ascii_uppercase();
        let remote = corpus.search("sam", 8)[0].clone();
        merge_results(&mut local, vec![duplicate, remote.clone(), remote]);
        assert_eq!(local.len(), ids.len() + 1);
        assert_eq!(
            local[..ids.len()]
                .iter()
                .map(|user| user.id.clone())
                .collect::<Vec<_>>(),
            ids
        );
        assert_eq!(local.last().unwrap().id.0, "sam");
    }
}
