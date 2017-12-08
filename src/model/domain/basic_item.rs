use super::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct BasicItem {
    pub id: ItemId,
    pub title: String,
    pub description: String,
    pub status: State,
    pub flagged: bool,
    pub project: Project,
    pub tags: Vec<Tag>,
}

impl Item for BasicItem {
    fn id(&self) -> &ItemId {
        &self.id
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
    fn tags(&self) -> &Vec<Tag> {
        &self.tags
    }
    fn project(&self) -> &Project {
        &self.project
    }
    fn parent(&self) -> Option<Box<Item>> {
        None
    }
}
