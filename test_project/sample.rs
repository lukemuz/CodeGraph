// Rust test file for CodeGraph

use std::collections::HashMap;
use std::error::Error;

// Struct definitions
#[derive(Debug, Clone)]
pub struct User {
    id: u32,
    name: String,
    email: String,
    age: Option<u32>,
}

#[derive(Debug)]
pub struct UserService {
    users: HashMap<u32, User>,
    next_id: u32,
}

// Implementation blocks
impl User {
    pub fn new(id: u32, name: String, email: String) -> Self {
        User {
            id,
            name,
            email,
            age: None,
        }
    }

    pub fn with_age(mut self, age: u32) -> Self {
        self.age = Some(age);
        self
    }

    pub fn get_display_name(&self) -> String {
        format!("{} (ID: {})", self.name, self.id)
    }

    pub fn is_adult(&self) -> bool {
        self.age.map_or(false, |age| age >= 18)
    }
}

impl UserService {
    pub fn new() -> Self {
        UserService {
            users: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn create_user(&mut self, name: String, email: String) -> User {
        let user = User::new(self.next_id, name, email);
        self.users.insert(self.next_id, user.clone());
        self.next_id += 1;
        user
    }

    pub fn get_user(&self, id: u32) -> Option<&User> {
        self.users.get(&id)
    }

    pub fn update_user(&mut self, id: u32, name: Option<String>, email: Option<String>) -> Option<&User> {
        if let Some(user) = self.users.get_mut(&id) {
            if let Some(new_name) = name {
                user.name = new_name;
            }
            if let Some(new_email) = email {
                user.email = new_email;
            }
            Some(user)
        } else {
            None
        }
    }

    pub fn delete_user(&mut self, id: u32) -> bool {
        self.users.remove(&id).is_some()
    }

    pub fn find_users_by_name(&self, query: &str) -> Vec<&User> {
        self.users
            .values()
            .filter(|user| user.name.to_lowercase().contains(&query.to_lowercase()))
            .collect()
    }

    pub fn get_all_users(&self) -> Vec<&User> {
        self.users.values().collect()
    }
}

// Trait definition
trait Validator {
    fn validate(&self) -> Result<(), String>;
}

impl Validator for User {
    fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Name cannot be empty".to_string());
        }
        
        if !is_valid_email(&self.email) {
            return Err("Invalid email format".to_string());
        }
        
        Ok(())
    }
}

// Standalone functions
pub fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}

pub fn process_users(users: Vec<User>) -> Vec<String> {
    users
        .into_iter()
        .filter(|user| user.is_adult())
        .map(|user| user.get_display_name())
        .collect()
}

// Generic function
pub fn find_by_id<T>(items: &HashMap<u32, T>, id: u32) -> Option<&T> {
    items.get(&id)
}

// Async function (requires tokio or async-std in real usage)
pub async fn fetch_user_data(id: u32) -> Result<User, Box<dyn Error>> {
    // Simulated async operation
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    Ok(User::new(id, "Fetched User".to_string(), "user@example.com".to_string()))
}

// Closure example
pub fn create_filter_fn(min_age: u32) -> impl Fn(&User) -> bool {
    move |user: &User| user.age.map_or(false, |age| age >= min_age)
}

// Macro usage
macro_rules! create_user {
    ($name:expr, $email:expr) => {
        User::new(0, $name.to_string(), $email.to_string())
    };
    ($name:expr, $email:expr, $age:expr) => {
        User::new(0, $name.to_string(), $email.to_string()).with_age($age)
    };
}

// Main function
fn main() {
    let mut service = UserService::new();
    
    // Create users
    let user1 = service.create_user("Alice".to_string(), "alice@example.com".to_string());
    let user2 = service.create_user("Bob".to_string(), "bob@example.com".to_string());
    
    println!("Created users: {:?}, {:?}", user1, user2);
    
    // Update user
    if let Some(updated) = service.update_user(user1.id, Some("Alice Smith".to_string()), None) {
        println!("Updated user: {:?}", updated);
    }
    
    // Search users
    let results = service.find_users_by_name("alice");
    println!("Search results: {:?}", results);
    
    // Validate user
    match user1.validate() {
        Ok(()) => println!("User is valid"),
        Err(e) => println!("Validation error: {}", e),
    }
    
    // Use macro
    let user3 = create_user!("Charlie", "charlie@example.com", 25);
    println!("Created with macro: {:?}", user3);
}

// Tests module
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new(1, "Test".to_string(), "test@example.com".to_string());
        assert_eq!(user.id, 1);
        assert_eq!(user.name, "Test");
    }

    #[test]
    fn test_email_validation() {
        assert!(is_valid_email("test@example.com"));
        assert!(!is_valid_email("invalid-email"));
    }

    #[test]
    fn test_user_service() {
        let mut service = UserService::new();
        let user = service.create_user("Test".to_string(), "test@example.com".to_string());
        
        assert_eq!(user.id, 1);
        assert!(service.get_user(1).is_some());
        assert!(service.delete_user(1));
        assert!(service.get_user(1).is_none());
    }
}