// JavaScript test file for CodeGraph

function hello() {
    return "Hello from JavaScript!";
}

const greet = (name) => {
    const greeting = hello();
    return `${greeting} Nice to meet you, ${name}!`;
};

class Calculator {
    constructor(initialValue = 0) {
        this.value = initialValue;
    }

    add(num) {
        this.value += num;
        return this.value;
    }

    subtract(num) {
        this.value -= num;
        return this.value;
    }

    multiply(num) {
        this.value *= num;
        return this.value;
    }

    divide(num) {
        if (num === 0) {
            throw new Error("Cannot divide by zero");
        }
        this.value /= num;
        return this.value;
    }

    reset() {
        this.value = 0;
        return this.value;
    }

    getValue() {
        return this.value;
    }
}

// Function using the class
function calculateSum(numbers) {
    const calc = new Calculator();
    
    for (const num of numbers) {
        calc.add(num);
    }
    
    return calc.getValue();
}

// Async function
async function fetchData(url) {
    try {
        const response = await fetch(url);
        const data = await response.json();
        return processData(data);
    } catch (error) {
        console.error("Error fetching data:", error);
        return null;
    }
}

function processData(data) {
    // Process the data
    return data.map(item => ({
        ...item,
        processed: true,
        timestamp: Date.now()
    }));
}

// Main function
function main() {
    const name = "Alice";
    console.log(greet(name));
    
    const numbers = [1, 2, 3, 4, 5];
    const sum = calculateSum(numbers);
    console.log(`Sum: ${sum}`);
    
    fetchData("https://api.example.com/data")
        .then(result => console.log(result));
}

// Export functions (for module usage)
module.exports = {
    hello,
    greet,
    Calculator,
    calculateSum,
    fetchData,
    processData,
    main
};