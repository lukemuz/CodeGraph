"""
Utility functions to test cross-file dependencies
"""

from main import hello_world, DataProcessor

def utility_function():
    """Function that calls functions from another file."""
    greeting = hello_world()
    return f"Utility says: {greeting}"

def create_processor(name):
    """Factory function."""
    return DataProcessor(name)

async def async_function():
    """Test async function parsing."""
    result = await some_async_operation()
    return process_async_result(result)

async def some_async_operation():
    """Another async function."""
    return "async result"

def process_async_result(result):
    """Process async results."""
    return f"Processed: {result}"

# Decorator test
def my_decorator(func):
    """Simple decorator."""
    def wrapper(*args, **kwargs):
        print(f"Calling {func.__name__}")
        return func(*args, **kwargs)
    return wrapper

@my_decorator
def decorated_function():
    """Function with decorator."""
    return utility_function()