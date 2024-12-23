#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Project {
    id: u64,
    title: String,
    description: String,
    goal_amount: u64,
    raised_amount: u64,
    creator: String,
    created_at: u64,
    updated_at: Option<u64>,
}

impl Storable for Project {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Project {
    const MAX_SIZE: u32 = 2048;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, Project, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct ProjectPayload {
    title: String,
    description: String,
    goal_amount: u64,
    creator: String,
}

#[ic_cdk::query]
fn get_project(id: u64) -> Result<Project, Error> {
    match _get_project(&id) {
        Some(project) => Ok(project),
        None => Err(Error::NotFound {
            msg: format!("A project with id={} not found", id),
        }),
    }
}

#[ic_cdk::update]
fn add_project(payload: ProjectPayload) -> Option<Project> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment id counter");
    let project = Project {
        id,
        title: payload.title,
        description: payload.description,
        goal_amount: payload.goal_amount,
        raised_amount: 0,
        creator: payload.creator,
        created_at: time(),
        updated_at: None,
    };
    do_insert(&project);
    Some(project)
}

#[ic_cdk::update]
fn update_project(id: u64, payload: ProjectPayload) -> Result<Project, Error> {
    match STORAGE.with(|storage| storage.borrow().get(&id)) {
        Some(mut project) => {
            project.title = payload.title;
            project.description = payload.description;
            project.goal_amount = payload.goal_amount;
            project.updated_at = Some(time());
            do_insert(&project);
            Ok(project)
        }
        None => Err(Error::NotFound {
            msg: format!(
                "Couldn't update a project with id={}. Project not found",
                id
            ),
        }),
    }
}

#[ic_cdk::update]
fn contribute_to_project(id: u64, amount: u64) -> Result<Project, Error> {
    match STORAGE.with(|storage| storage.borrow().get(&id)) {
        Some(mut project) => {
            project.raised_amount += amount;
            project.updated_at = Some(time());
            do_insert(&project);
            Ok(project)
        }
        None => Err(Error::NotFound {
            msg: format!(
                "Couldn't contribute to project with id={}. Project not found",
                id
            ),
        }),
    }
}

fn do_insert(project: &Project) {
    STORAGE.with(|storage| storage.borrow_mut().insert(project.id, project.clone()));
}

#[ic_cdk::update]
fn delete_project(id: u64) -> Result<Project, Error> {
    match STORAGE.with(|storage| storage.borrow_mut().remove(&id)) {
        Some(project) => Ok(project),
        None => Err(Error::NotFound {
            msg: format!(
                "Couldn't delete a project with id={}. Project not found.",
                id
            ),
        }),
    }
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}

fn _get_project(id: &u64) -> Option<Project> {
    STORAGE.with(|storage| storage.borrow().get(id))
}

ic_cdk::export_candid!();
