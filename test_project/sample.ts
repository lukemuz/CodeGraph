// TypeScript test file for CodeGraph

interface User {
    id: number;
    name: string;
    email: string;
    age?: number;
}

interface ApiResponse<T> {
    data: T;
    status: number;
    message?: string;
}

class UserService {
    private users: Map<number, User>;
    private nextId: number;

    constructor() {
        this.users = new Map();
        this.nextId = 1;
    }

    createUser(name: string, email: string, age?: number): User {
        const user: User = {
            id: this.nextId++,
            name,
            email,
            age
        };
        
        this.users.set(user.id, user);
        return user;
    }

    getUser(id: number): User | undefined {
        return this.users.get(id);
    }

    updateUser(id: number, updates: Partial<User>): User | null {
        const user = this.getUser(id);
        if (!user) {
            return null;
        }

        const updatedUser = { ...user, ...updates };
        this.users.set(id, updatedUser);
        return updatedUser;
    }

    deleteUser(id: number): boolean {
        return this.users.delete(id);
    }

    getAllUsers(): User[] {
        return Array.from(this.users.values());
    }

    findUsersByName(name: string): User[] {
        const users = this.getAllUsers();
        return users.filter(user => 
            user.name.toLowerCase().includes(name.toLowerCase())
        );
    }
}

// Generic function
function createApiResponse<T>(data: T, status: number, message?: string): ApiResponse<T> {
    return {
        data,
        status,
        message
    };
}

// Async function with type annotations
async function fetchUserData(userId: number): Promise<ApiResponse<User | null>> {
    try {
        // Simulated API call
        await delay(100);
        
        const service = new UserService();
        const user = service.getUser(userId);
        
        if (user) {
            return createApiResponse(user, 200, "User found");
        } else {
            return createApiResponse(null, 404, "User not found");
        }
    } catch (error) {
        return createApiResponse(null, 500, "Internal server error");
    }
}

// Helper function
function delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
}

// Arrow function with type annotations
const validateEmail = (email: string): boolean => {
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return emailRegex.test(email);
};

// Main execution function
async function main(): Promise<void> {
    const service = new UserService();
    
    // Create users
    const user1 = service.createUser("Alice", "alice@example.com", 30);
    const user2 = service.createUser("Bob", "bob@example.com");
    
    console.log("Created users:", user1, user2);
    
    // Update user
    const updated = service.updateUser(user1.id, { age: 31 });
    console.log("Updated user:", updated);
    
    // Search users
    const searchResults = service.findUsersByName("alice");
    console.log("Search results:", searchResults);
    
    // Fetch user data
    const response = await fetchUserData(user1.id);
    console.log("API Response:", response);
    
    // Validate email
    const isValid = validateEmail(user1.email);
    console.log(`Email validation for ${user1.email}: ${isValid}`);
}

// Export for module usage
export {
    User,
    ApiResponse,
    UserService,
    createApiResponse,
    fetchUserData,
    delay,
    validateEmail,
    main
};