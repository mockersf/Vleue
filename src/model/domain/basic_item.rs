use super::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct BasicItem {
    pub uid: super::super::UserId,
    pub id: ItemId,
    pub title: String,
    pub description: String,
    pub status: State,
    pub flagged: bool,
    pub project_id: ProjectId,
    //pub tags: Vec<TagId>,
}

impl Item for BasicItem {
    fn id(&self) -> &ItemId {
        &self.id
    }
    fn user_id(&self) -> &super::super::UserId {
        &self.uid
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn status(&self) -> &State {
        &self.status
    }
    fn flagged(&self) -> bool {
        self.flagged
    }
    /*fn tags(&self) -> &Vec<TagId> {
        &self.tags
    }*/
    fn project(&self) -> &ProjectId {
        &self.project_id
    }
    fn parent(&self) -> Option<Box<ItemId>> {
        None
    }
}
