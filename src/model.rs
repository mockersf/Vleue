#[derive(Serialize, Deserialize, Debug)]
pub struct Todo {
    task: Task
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Task {
    pub id: Option<String>,
    pub task: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TodoList {
    pub todos: Vec<Task>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Application {
    pub app_id: String,
    pub app_secret: Option<String>,
}
