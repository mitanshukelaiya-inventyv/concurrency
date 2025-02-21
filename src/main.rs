// Example : 
// Task Definition: Implement a Multithreaded Periodic Logger with Shared Configuration
//  Setting Up Periodic Tasks in Threads
//  Utilizing lazy_static! for Lazy Initialization
//  Sharing Data Across Threads with Arc and RwLock
//  Combining Periodic Tasks with Shared Lazy-Initialized Data
 
 
// exp . 
// one thread handle axum server. 
// one schedular updating user data.
// one scheduler is generating call department wise. 
// one is checking that user data and call data and assign(just log that user asisgn to which id number call ) call user's department wise.   
// create api that return all user , all calls , all assigned task (can stored in one file)



use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    thread::{self, sleep},
    time::Duration,
};
use lazy_static::lazy_static;
use axum::{response::IntoResponse, routing::get, Json, Router};
use rand::Rng;
use serde::{Deserialize, Serialize};

lazy_static! {
    static ref USERS: Arc<RwLock<Vec<User>>> = Arc::new(RwLock::new(get_users_dept_wise()));
    static ref CALLS: Arc<RwLock<Vec<Call>>> = Arc::new(RwLock::new(Vec::new()));
    static ref ASSIGNED_TASKS: Arc<RwLock<Vec<AssignedTask>>> = Arc::new(RwLock::new(Vec::new()));
    static ref STATUS_MAP: Arc<RwLock<HashMap<Department, Vec<User>>>> = Arc::new(RwLock::new(HashMap::new())); //Not Yet Used
}


#[tokio::main]
async fn main() {
    let call_generate_call_clone = Arc::clone(&CALLS);

    let change_status_employee_clone = Arc::clone(&USERS);

    let assign_call_clone = Arc::clone(&CALLS);
    let assign_employee_clone = Arc::clone(&USERS);
    let assign_assigned_task_clone = Arc::clone(&ASSIGNED_TASKS);

    let change_status = thread::spawn(move || loop {
        {
            let mut users_write = change_status_employee_clone.write().unwrap();
            if !users_write.is_empty() {
                let random_index = rand::rng().random_range(0..users_write.len());
                let user = &mut users_write[random_index];
                user.status = UserStatus::get_random_user_status();
                println!("Randomly updated user: {:?}", user);
            }
        }
        sleep(Duration::from_millis(7000));
    });

    let call_generate = thread::spawn(move || {
        let mut cnt = 0;
        loop {
            let new_call = Call {
                id: cnt,
                department: Department::get_random_department(),
            };

            {
                call_generate_call_clone.write().unwrap().push(new_call);
            }
            cnt+=1;
            sleep(Duration::from_millis(2000));
        }
    });

    let assign = thread::spawn(move || loop {
        {
            let mut assign_call_guard = assign_call_clone.write().unwrap();
            let mut assign_employee_guard = assign_employee_clone.write().unwrap();
            let mut assign_assigned_task_clone_guard = assign_assigned_task_clone.write().unwrap();
            if assign_call_guard.len() > 0 {
                let front_call = assign_call_guard.remove(0);
                let dept = front_call.department.clone();

                //let dept_vec = status_map.get(&dept);
                println!("{:?}", front_call);
                for emp in assign_employee_guard.iter_mut() {
                    
                    if emp.department == dept && emp.status == UserStatus::Available {
                        emp.status = UserStatus::OnCall;
                        println!("call {} assigned to Employee: {}", front_call.id, emp.id);
                        emp.status = UserStatus::Available;
                        assign_assigned_task_clone_guard.push(AssignedTask{
                            call_id: front_call.id,
                            user_id: emp.id,
                            status: String::from("Accepted")
                        });
                    } else if emp.department == dept {
                        println!(
                            "call {} dropped.... Current User status: {:?}",
                            front_call.id, emp.status
                        );
                        assign_assigned_task_clone_guard.push(AssignedTask{
                            call_id: front_call.id,
                            user_id: emp.id,
                            status: String::from("Rejected")
                        });
                    }
                }
            }
        }
        sleep(Duration::from_millis(1000));
    });

    let server = tokio::spawn(async move {
        println!("Creating server...");

        let app = Router::new()
        .route("/users", get(get_users))
        .route("/calls", get(get_calls))
        .route("/tasks", get(get_tasks));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:7878")
            .await
            .unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    server.await.unwrap();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i32,
    name: String,
    department: Department,
    status: UserStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Call {
    id: i32,
    department: Department,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssignedTask {
    call_id: i32,
    user_id: i32,
    status:String
}

// HashMap keys in Rust must implement both the Eq and Hash traits. These traits are required to check for equality and to hash the keys for storage and lookup
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
enum UserStatus {
    OnCall,
    Available,
    Break,
    LoggedOut,
}

impl UserStatus {
    fn iterator() -> impl Iterator<Item = UserStatus> {
        [
            UserStatus::OnCall,
            UserStatus::Available,
            UserStatus::Break,
            UserStatus::LoggedOut,
        ]
        .iter()
        .cloned()
    }

    fn get_random_user_status() -> UserStatus {
        let random_index = rand::rng().random_range(0..=3);
        Self::iterator().nth(random_index).unwrap()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
enum Department {
    Sales,
    Renewal,
    Audit,
    Developer,
    HR,
}

impl Department {
    fn iterator() -> impl Iterator<Item = Department> {
        [
            Department::Sales,
            Department::Renewal,
            Department::Audit,
            Department::Developer,
            Department::HR,
        ]
        .iter()
        .cloned()
    }

    fn get_random_department() -> Department {
        let random_index = rand::rng().random_range(0..=4);
        Self::iterator().nth(random_index).unwrap()
    }
}

fn get_users_dept_wise() -> Vec<User> {
    let mut users: Vec<User> = vec![];
    let mut cnt = 1;
    for dept in Department::iterator() {
        let new_user = User {
            id: cnt,
            name: String::from(format!("Mahesh_{:?}", dept.clone())),
            department: dept.clone(),
            status: UserStatus::Available,
        };
        cnt += 1;
        users.push(new_user);
    }
    users
}

async fn get_users() -> Json<Vec<User>> {
    let server_user_clone = Arc::clone(&USERS);
    let server_clone_guard = server_user_clone.read().unwrap();

    Json(server_clone_guard.clone())
}

async fn get_calls() -> impl IntoResponse {
    let server_call_clone = Arc::clone(&CALLS);
    let server_call_guard = server_call_clone.read().unwrap();

    Json(server_call_guard.clone())
}

async fn get_tasks() -> impl IntoResponse {
    let server_task_clone = Arc::clone(&ASSIGNED_TASKS);
    let server_task_guard = server_task_clone.read().unwrap();

    Json(server_task_guard.clone())
}
