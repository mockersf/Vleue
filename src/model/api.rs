#[derive(Serialize, Deserialize, Debug)]
pub struct ItemList {
    pub items: Vec<super::Item>,
}
