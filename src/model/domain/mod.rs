//use chrono;

pub mod basic_item;

typed_id!(ItemId);
/*
typed_id!(CategoryId);
typed_id!(ProjectId);
typed_id!(ContextId);
typed_id!(CostCategoryId);
*/

pub trait Item {
    fn id(&self) -> &ItemId;
    fn title(&self) -> &str;
    fn description(&self) -> &str;
    fn status(&self) -> &State;
    fn flagged(&self) -> bool;
//    fn costs(&self) -> &Vec<Cost>;
    fn tags(&self) -> &Vec<Tag>;
    fn project(&self) -> &Project;
//    fn contexts(&self) -> &Vec<Context>;
    fn parent(&self) -> Option<Box<Item>>;
/*    fn due(&self) -> Option<chrono::DateTime<chrono::Utc>>;
    fn defer(&self) -> Option<chrono::DateTime<chrono::Utc>>;
    fn repeat(&self) -> Option<Repeat>;*/
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
    pub from: State,
    pub to: State,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Workflow {
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
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