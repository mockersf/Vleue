#[derive(Serialize, Deserialize, Debug)]
pub struct ItemList<T = super::basic_item::BasicItem>
    where T: super::Item
{
    pub items: Vec<T>,
}
