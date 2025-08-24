use codegraph::cli::Indexer;
use codegraph::graph::CodeGraph;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_python_project_indexing() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    let python_code = r#"
def hello_world():
    """Simple hello world function."""
    return "Hello, World!"

def greet_user(name):
    """Greet a specific user."""
    return hello_world() + f" Nice to meet you, {name}!"

class Calculator:
    def add(self, a, b):
        return a + b
    
    def multiply(self, a, b):
        return self.add(a, 0) * b
"#;
    
    let python_file = project_path.join("main.py");
    fs::write(&python_file, python_code).unwrap();
    
    let indexer = Indexer::new().unwrap();
    let index_path = project_path.join("index.bin");
    
    indexer.index_project(project_path, &index_path, false).unwrap();
    
    assert!(index_path.exists());
    
    let graph = indexer.load_index(&index_path).unwrap();
    
    assert!(graph.graph.node_count() > 0);
    
    let hello_world_found = graph.find_exact("hello_world");
    assert!(hello_world_found.is_some());
    
    let greet_user_found = graph.find_exact("greet_user");
    assert!(greet_user_found.is_some());
    
    let calculator_add_found = graph.find_exact("Calculator.add");
    assert!(calculator_add_found.is_some());
}

#[test]
fn test_javascript_project_indexing() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    let js_code = r#"
function fetchData(url) {
    return fetch(url).then(response => response.json());
}

const processData = async (data) => {
    return data.map(item => item.name);
};

class DataProcessor {
    constructor() {
        this.cache = new Map();
    }
    
    async process(url) {
        if (this.cache.has(url)) {
            return this.cache.get(url);
        }
        
        const data = await fetchData(url);
        const processed = await processData(data);
        this.cache.set(url, processed);
        return processed;
    }
}
"#;
    
    let js_file = project_path.join("main.js");
    fs::write(&js_file, js_code).unwrap();
    
    let indexer = Indexer::new().unwrap();
    let index_path = project_path.join("index.bin");
    
    indexer.index_project(project_path, &index_path, false).unwrap();
    
    assert!(index_path.exists());
    
    let graph = indexer.load_index(&index_path).unwrap();
    
    assert!(graph.graph.node_count() > 0);
    
    let fetch_data_found = graph.find_exact("fetchData");
    assert!(fetch_data_found.is_some());
    
    let process_data_found = graph.find_exact("processData");
    assert!(process_data_found.is_some());
}

#[test]
fn test_function_resolution() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    let python_code = r#"
def authentication_middleware(request):
    return validate_token(request.headers.get('Authorization'))

def validate_token(token):
    return token is not None and len(token) > 10

def process_payment(amount, token):
    if not authentication_middleware({'headers': {'Authorization': token}}):
        raise ValueError("Invalid token")
    return f"Processing ${amount}"
"#;
    
    let python_file = project_path.join("auth.py");
    fs::write(&python_file, python_code).unwrap();
    
    let indexer = Indexer::new().unwrap();
    let index_path = project_path.join("index.bin");
    
    indexer.index_project(project_path, &index_path, false).unwrap();
    let graph = indexer.load_index(&index_path).unwrap();
    
    let auth_middleware = graph.find_by_pattern("auth");
    assert!(!auth_middleware.is_empty());
    
    let payment_funcs = graph.find_by_pattern("payment");
    assert!(!payment_funcs.is_empty());
}