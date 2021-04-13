//! Multiple scenarios that are performed to test sync operations correctly work
//!
//! This module creates test data.
//! To do so, "scenarii" are defined. A scenario contains an inital state before sync, changes made either on the local or remote side, then the expected final state that should be present in both sources after sync.
//!
//! This module builds actual CalDAV sources (actually [`crate::cache::Cache`]s, that can also mock what would be [`crate::client::Client`]s in a real program) and [`crate::provider::Provider]`s that contain this data
//!
//! This module can also check the sources after a sync contain the actual data we expect
#![cfg(feature = "local_calendar_mocks_remote_calendars")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::error::Error;

use my_tasks::calendar::CalendarId;
use my_tasks::calendar::SupportedComponents;
use my_tasks::traits::CalDavSource;
use my_tasks::traits::BaseCalendar;
use my_tasks::traits::CompleteCalendar;
use my_tasks::traits::DavCalendar;
use my_tasks::cache::Cache;
use my_tasks::Item;
use my_tasks::ItemId;
use my_tasks::SyncStatus;
use my_tasks::Task;
use my_tasks::calendar::cached_calendar::CachedCalendar;
use my_tasks::Provider;
use my_tasks::mock_behaviour::MockBehaviour;

pub enum LocatedState {
    /// Item does not exist yet or does not exist anymore
    None,
    /// Item is only in the local source
    Local(ItemState),
    /// Item is only in the remote source
    Remote(ItemState),
    /// Item is synced at both locations,
    BothSynced(ItemState),
}

pub struct ItemState {
    // TODO: if/when this crate supports Events as well, we could add such events here
    /// The calendar it is in
    calendar: CalendarId,
    /// Its name
    name: String,
    /// Its completion status
    completed: bool,
}

pub enum ChangeToApply {
    Rename(String),
    SetCompletion(bool),
    Create(CalendarId, Item),
    /// "remove" means "mark for deletion" in the local calendar, or "immediately delete" on the remote calendar
    Remove,
    // ChangeCalendar(CalendarId) is useless, as long as changing a calendar is implemented as "delete in one calendar and re-create it in another one"
}


pub struct ItemScenario {
    id: ItemId,
    initial_state: LocatedState,
    local_changes_to_apply:  Vec<ChangeToApply>,
    remote_changes_to_apply: Vec<ChangeToApply>,
    after_sync: LocatedState,
}

/// Generate the scenarii required for the following test:
/// * At the last sync: both sources had A, B, C, D, E, F, G, H, I, J, K, L, M✓, N✓, O✓, P✓ at last sync
/// * Before the newer sync, this will be the content of the sources:
///     * cache:  A, B,    D', E,  F'', G , H✓, I✓, J✓,        M,  N✓, O, P' ,    R
///     * server: A,    C, D,  E', F',  G✓, H , I',      K✓,    M✓, N , O, P✓,  Q
///
/// Hence, here is the expected result after the sync:
///     * both:   A,       D', E', F',  G✓, H✓, I',      K✓,   M, N, O, P', Q, R
///
/// Notes:
/// * X': name has been modified since the last sync
/// * X'/X'': name conflict
/// * X✓: task has been marked as completed
pub fn scenarii_basic() -> Vec<ItemScenario> {
    let mut tasks = Vec::new();

    let main_cal = CalendarId::from("https://some.calend.ar/main/".parse().unwrap());

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task A"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task A"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task B"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: vec![ChangeToApply::Remove],
            after_sync: LocatedState::None,
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task C"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::Remove],
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::None,
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task D"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::Rename(String::from("Task D, locally renamed"))],
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task D, locally renamed"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task E"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: vec![ChangeToApply::Rename(String::from("Task E, remotely renamed"))],
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task E, remotely renamed"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task F"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::Rename(String::from("Task F, locally renamed"))],
            remote_changes_to_apply: vec![ChangeToApply::Rename(String::from("Task F, remotely renamed"))],
            // Conflict: the server wins
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task F, remotely renamed"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task G"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: vec![ChangeToApply::SetCompletion(true)],
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task G"),
                completed: true,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task H"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::SetCompletion(true)],
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task H"),
                completed: true,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task I"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::SetCompletion(true)],
            remote_changes_to_apply: vec![ChangeToApply::Rename(String::from("Task I, remotely renamed"))],
            // Conflict, the server wins
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task I, remotely renamed"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task J"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::SetCompletion(true)],
            remote_changes_to_apply: vec![ChangeToApply::Remove],
            after_sync: LocatedState::None,
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task K"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::Remove],
            remote_changes_to_apply: vec![ChangeToApply::SetCompletion(true)],
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task K"),
                completed: true,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task L"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::Remove],
            remote_changes_to_apply: vec![ChangeToApply::Remove],
            after_sync: LocatedState::None,
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task M"),
                completed: true,
            }),
            local_changes_to_apply: vec![ChangeToApply::SetCompletion(false)],
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task M"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task N"),
                completed: true,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: vec![ChangeToApply::SetCompletion(false)],
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task N"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task O"),
                completed: true,
            }),
            local_changes_to_apply:  vec![ChangeToApply::SetCompletion(false)],
            remote_changes_to_apply: vec![ChangeToApply::SetCompletion(false)],
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task O"),
                completed: false,
            }),
        }
    );

    let id_p = ItemId::random();
    tasks.push(
        ItemScenario {
            id: id_p.clone(),
            initial_state: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task P"),
                completed: true,
            }),
            local_changes_to_apply: vec![
                ChangeToApply::Rename(String::from("Task P, locally renamed and un-completed")),
                ChangeToApply::SetCompletion(false),
            ],
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task P, locally renamed and un-completed"),
                completed: false,
            }),
        }
    );

    let id_q = ItemId::random();
    tasks.push(
        ItemScenario {
            id: id_q.clone(),
            initial_state: LocatedState::None,
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: vec![ChangeToApply::Create(main_cal.clone(), Item::Task(
                Task::new(String::from("Task Q, created on the server"), id_q, SyncStatus::random_synced(), false )
            ))],
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task Q, created on the server"),
                completed: false,
            }),
        }
    );

    let id_r = ItemId::random();
    tasks.push(
        ItemScenario {
            id: id_r.clone(),
            initial_state: LocatedState::None,
            local_changes_to_apply: vec![ChangeToApply::Create(main_cal.clone(), Item::Task(
                Task::new(String::from("Task R, created locally"), id_r, SyncStatus::NotSynced, false )
            ))],
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task R, created locally"),
                completed: false,
            }),
        }
    );

    tasks
}

/// This scenario basically checks a first sync to an empty local cache
pub fn scenarii_first_sync_to_local() -> Vec<ItemScenario> {
    let mut tasks = Vec::new();

    let cal1 = CalendarId::from("https://some.calend.ar/first/".parse().unwrap());
    let cal2 = CalendarId::from("https://some.calend.ar/second/".parse().unwrap());

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::Remote( ItemState{
                calendar: cal1.clone(),
                name: String::from("Task A1"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: cal1.clone(),
                name: String::from("Task A1"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::Remote( ItemState{
                calendar: cal2.clone(),
                name: String::from("Task A2"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: cal2.clone(),
                name: String::from("Task A2"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::Remote( ItemState{
                calendar: cal1.clone(),
                name: String::from("Task B1"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: cal1.clone(),
                name: String::from("Task B1"),
                completed: false,
            }),
        }
    );

    tasks
}

/// This scenario basically checks a first sync to an empty server
pub fn scenarii_first_sync_to_server() -> Vec<ItemScenario> {
    let mut tasks = Vec::new();

    let cal3 = CalendarId::from("https://some.calend.ar/third/".parse().unwrap());
    let cal4 = CalendarId::from("https://some.calend.ar/fourth/".parse().unwrap());

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::Local( ItemState{
                calendar: cal3.clone(),
                name: String::from("Task A3"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: cal3.clone(),
                name: String::from("Task A3"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::Local( ItemState{
                calendar: cal4.clone(),
                name: String::from("Task A4"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: cal4.clone(),
                name: String::from("Task A4"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::Local( ItemState{
                calendar: cal3.clone(),
                name: String::from("Task B3"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: cal3.clone(),
                name: String::from("Task B3"),
                completed: false,
            }),
        }
    );

    tasks
}


/// This scenario tests a task added and deleted before a sync happens
pub fn scenarii_transient_task() -> Vec<ItemScenario> {
    let mut tasks = Vec::new();

    let cal = CalendarId::from("https://some.calend.ar/transient/".parse().unwrap());

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            initial_state: LocatedState::Local( ItemState{
                calendar: cal.clone(),
                name: String::from("A task, so that the calendar actually exists"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: cal.clone(),
                name: String::from("A task, so that the calendar actually exists"),
                completed: false,
            }),
        }
    );

    let id_transient = ItemId::random();
    tasks.push(
        ItemScenario {
            id: id_transient.clone(),
            initial_state: LocatedState::None,
            local_changes_to_apply: vec![
                ChangeToApply::Create(cal, Item::Task(
                    Task::new(String::from("A transient task that will be deleted before the sync"), id_transient, SyncStatus::NotSynced, false )
                )),

                ChangeToApply::Rename(String::from("A new name")),
                ChangeToApply::SetCompletion(true),
                ChangeToApply::Remove,
            ],
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::None,
        }
    );

    tasks
}


/// Build a `Provider` that contains the data (defined in the given scenarii) before sync
pub async fn populate_test_provider_before_sync(scenarii: &[ItemScenario], mock_behaviour: Arc<Mutex<MockBehaviour>>) -> Provider<Cache, CachedCalendar, Cache, CachedCalendar> {
    let mut provider = populate_test_provider(scenarii, mock_behaviour, false).await;
    apply_changes_on_provider(&mut provider, scenarii).await;
    provider
}

/// Build a `Provider` that contains the data (defined in the given scenarii) after sync
pub async fn populate_test_provider_after_sync(scenarii: &[ItemScenario], mock_behaviour: Arc<Mutex<MockBehaviour>>) -> Provider<Cache, CachedCalendar, Cache, CachedCalendar> {
    populate_test_provider(scenarii, mock_behaviour, true).await
}

async fn populate_test_provider(scenarii: &[ItemScenario], mock_behaviour: Arc<Mutex<MockBehaviour>>, populate_for_final_state: bool) -> Provider<Cache, CachedCalendar, Cache, CachedCalendar> {
    let mut local = Cache::new(&PathBuf::from(String::from("test_cache_local/")));
    let mut remote = Cache::new(&PathBuf::from(String::from("test_cache_remote/")));
    remote.set_mock_behaviour(Some(mock_behaviour));

    // Create the initial state, as if we synced both sources in a given state
    for item in scenarii {
        let required_state = if populate_for_final_state { &item.after_sync } else { &item.initial_state };
        let (state, sync_status) = match required_state {
            LocatedState::None => continue,
            LocatedState::Local(s) => {
                assert!(populate_for_final_state == false, "You are not supposed to expect an item in this state after sync");
                (s, SyncStatus::NotSynced)
            },
            LocatedState::Remote(s) => {
                assert!(populate_for_final_state == false, "You are not supposed to expect an item in this state after sync");
                (s, SyncStatus::random_synced())
            }
            LocatedState::BothSynced(s) => (s, SyncStatus::random_synced()),
        };

        let new_item = Item::Task(
            Task::new(
                state.name.clone(),
                item.id.clone(),
                sync_status,
                state.completed,
            ));

        match required_state {
            LocatedState::None => panic!("Should not happen, we've continued already"),
            LocatedState::Local(s) => {
                get_or_insert_calendar(&mut local,  &s.calendar).await.unwrap().lock().unwrap().add_item(new_item).await.unwrap();
            },
            LocatedState::Remote(s) => {
                get_or_insert_calendar(&mut remote, &s.calendar).await.unwrap().lock().unwrap().add_item(new_item).await.unwrap();
            },
            LocatedState::BothSynced(s) => {
                get_or_insert_calendar(&mut local,  &s.calendar).await.unwrap().lock().unwrap().add_item(new_item.clone()).await.unwrap();
                get_or_insert_calendar(&mut remote, &s.calendar).await.unwrap().lock().unwrap().add_item(new_item).await.unwrap();
            },
        }
    }
    Provider::new(remote, local)
}

/// Apply `local_changes_to_apply` and `remote_changes_to_apply` to a provider that contains data before sync
async fn apply_changes_on_provider(provider: &mut Provider<Cache, CachedCalendar, Cache, CachedCalendar>, scenarii: &[ItemScenario]) {
    // Apply changes to each item
    for item in scenarii {
        let initial_calendar_id = match &item.initial_state {
            LocatedState::None => None,
            LocatedState::Local(state) => Some(state.calendar.clone()),
            LocatedState::Remote(state) => Some(state.calendar.clone()),
            LocatedState::BothSynced(state) => Some(state.calendar.clone()),
        };

        let mut calendar_id = initial_calendar_id.clone();
        for local_change in &item.local_changes_to_apply {
            calendar_id = Some(apply_change(provider.local(), calendar_id, &item.id, local_change, false).await);
        }

        let mut calendar_id = initial_calendar_id;
        for remote_change in &item.remote_changes_to_apply {
            calendar_id = Some(apply_change(provider.remote(), calendar_id, &item.id, remote_change, true).await);
        }
    }
}

async fn get_or_insert_calendar(source: &mut Cache, id: &CalendarId)
    -> Result<Arc<Mutex<CachedCalendar>>, Box<dyn Error>>
{
    match source.get_calendar(id).await {
        Some(cal) => Ok(cal),
        None => {
            let new_name = format!("Test calendar for ID {}", id);
            let supported_components = SupportedComponents::TODO;

            source.create_calendar(
                id.clone(),
                new_name.to_string(),
                supported_components,
            ).await
        }
    }
}

/// Apply a single change on a given source, and returns the calendar ID that was modified
async fn apply_change<S, C>(source: &S, calendar_id: Option<CalendarId>, item_id: &ItemId, change: &ChangeToApply, is_remote: bool) -> CalendarId
where
    S: CalDavSource<C>,
    C: CompleteCalendar + DavCalendar, // in this test, we're using a calendar that mocks both kinds
{
    match calendar_id {
        Some(cal) => {
            apply_changes_on_an_existing_item(source, &cal, item_id, change, is_remote).await;
            cal
        },
        None => {
            create_test_item(source, change).await
        },
    }
}

async fn apply_changes_on_an_existing_item<S, C>(source: &S, calendar_id: &CalendarId, item_id: &ItemId, change: &ChangeToApply, is_remote: bool)
where
    S: CalDavSource<C>,
    C: CompleteCalendar + DavCalendar, // in this test, we're using a calendar that mocks both kinds
{
    let cal = source.get_calendar(calendar_id).await.unwrap();
    let mut cal = cal.lock().unwrap();
    let task = cal.get_item_by_id_mut(item_id).await.unwrap().unwrap_task_mut();

    match change {
        ChangeToApply::Rename(new_name) => {
            if is_remote {
                task.mock_remote_calendar_set_name(new_name.clone());
            } else {
                task.set_name(new_name.clone());
            }
        },
        ChangeToApply::SetCompletion(new_status) => {
            if is_remote {
                task.mock_remote_calendar_set_completed(new_status.clone());
            } else {
                task.set_completed(new_status.clone());
            }
        },
        ChangeToApply::Remove => {
            match is_remote {
                false => cal.mark_for_deletion(item_id).await.unwrap(),
                true => cal.delete_item(item_id).await.unwrap(),
            };
        },
        ChangeToApply::Create(_calendar_id, _item) => {
            panic!("This function only handles already existing items");
        },
    }
}

/// Create an item, and returns the calendar ID it was inserted in
async fn create_test_item<S, C>(source: &S, change: &ChangeToApply) -> CalendarId
where
    S: CalDavSource<C>,
    C: CompleteCalendar + DavCalendar, // in this test, we're using a calendar that mocks both kinds
{
    match change {
        ChangeToApply::Rename(_) |
        ChangeToApply::SetCompletion(_) |
        ChangeToApply::Remove => {
            panic!("This function only creates items that do not exist yet");
        }
        ChangeToApply::Create(calendar_id, item) => {
            let cal = source.get_calendar(calendar_id).await.unwrap();
            cal.lock().unwrap().add_item(item.clone()).await.unwrap();
            calendar_id.clone()
        },
    }
}
