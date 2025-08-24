#!/usr/bin/env python3
"""
Test Python project for CodeGraph parsing
"""

def hello_world():
    """Simple function with no dependencies."""
    return "Hello, World!"

def new_test_function():
    """This function was added after indexing."""
    return "Testing lazy refresh!"

def greet_user(name):
    """Function that calls another function."""
    greeting = hello_world()
    return f"{greeting} Nice to meet you, {name}!"

def process_data(data_list):
    """Function with multiple calls."""
    cleaned = clean_data(data_list)
    validated = validate_data(cleaned)
    return format_output(validated)

def clean_data(data):
    """Data cleaning function."""
    return [item.strip().lower() for item in data if item]

def validate_data(data):
    """Data validation function."""
    return [item for item in data if is_valid(item)]

def is_valid(item):
    """Simple validation."""
    return len(item) > 2

def format_output(data):
    """Format the final output."""
    return {"results": data, "count": len(data)}

class DataProcessor:
    """Class with methods to test method parsing."""
    
    def __init__(self, name):
        self.name = name
        self.cache = {}
    
    def process(self, data):
        """Method that calls other methods."""
        if self._is_cached(data):
            return self._get_from_cache(data)
        
        result = self._do_processing(data)
        self._store_in_cache(data, result)
        return result
    
    def _is_cached(self, data):
        """Private method."""
        key = self._generate_key(data)
        return key in self.cache
    
    def _get_from_cache(self, data):
        """Get from cache."""
        key = self._generate_key(data)
        return self.cache[key]
    
    def _store_in_cache(self, data, result):
        """Store in cache."""
        key = self._generate_key(data)
        self.cache[key] = result
    
    def _generate_key(self, data):
        """Generate cache key."""
        return hash(str(data))
    
    def _do_processing(self, data):
        """Do the actual processing."""
        # Call module-level function
        return process_data(data)

# Function with complex calls
def main():
    """Main function that orchestrates everything."""
    processor = DataProcessor("main_processor")
    test_data = ["  Hello  ", "World", "Test", "", "A"]
    
    # Direct function call
    simple_result = greet_user("Alice")
    
    # Method call
    complex_result = processor.process(test_data)
    
    print(simple_result)
    print(complex_result)

if __name__ == "__main__":
    main()
def another_new_function():
    """Added to test freshness checking."""
    return "Freshness test!"
