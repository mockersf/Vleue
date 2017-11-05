use std::collections::HashMap;
use chrono;

/*typed_id!(ItemId);
typed_id!(CategoryId);
typed_id!(ProjectId);
typed_id!(ContextId);
typed_id!(CostCategoryId);*/

#[derive(Serialize, Deserialize, Debug)]
pub struct Item {
//    pub id: ItemId,
    pub title: String,
    pub description: String,
    pub status: String,
    pub flagged: bool,
    pub fields: HashMap<String, String>,
    pub costs: Vec<Cost>,
    pub tags: Vec<Tag>,
    pub projects: Project,
    pub contexts: Option<Context>,
    pub due: Option<chrono::DateTime<chrono::Utc>>,
    pub defer: Option<chrono::DateTime<chrono::Utc>>,
    pub repeat: Option<Repeat>,
    pub parent: Option<Box<Item>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ItemType {
    Task,
    Item,
    Bug,
    Todo,
    Birthday,
    Category,
    Free { name: String}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CostCategory {
//    pub id: CostCategoryId,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Cost {
    pub category: CostCategory,
    pub cost: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tag {
//    pub id: CategoryId,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CostInfo {
    pub categories: Vec<CostCategory>,
    pub unit: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Transition {
    pub name: String,
    pub next: State,    
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub name: String,
    pub transitions: Vec<Transition>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Workflow {
    pub states: Vec<State>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
//    pub id: ProjectId,
    pub name: String,
    pub costs_info: CostInfo,
    pub workflow: Workflow,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Context {
//    pub id: ContextId,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Repeat {

}