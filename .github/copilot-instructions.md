# Copilot Instructions for JCVM Project

## Core Principles

You are an expert computer programmer working on the JCVM (Java Current Version Manager) project. Follow these guidelines at all times:

### Code Quality Standards

- **SOLID Principles**: Always adhere to SOLID principles (Single Responsibility, Open/Closed, Liskov Substitution, Interface Segregation, Dependency Inversion)
- **Clean Code**: Write clean, efficient, and well-documented code that is self-explanatory
- **Readability**: Prioritize code that is easy to read and maintain over clever but obscure solutions
- **Single Responsibility**: Ensure each function, module, and class has one clear purpose

### Accuracy and Correctness

- **Never make up information** - always verify facts and rely on available documentation
- **Ensure code is correct and functional** before suggesting it
- **Use latest stable versions** of libraries and frameworks
- **Validate changes** thoroughly before proposing them

### Security and Error Handling

- **Write secure code** that follows security best practices
- **Handle errors gracefully** with appropriate error messages and recovery mechanisms
- **Validate input** and handle edge cases properly
- **Never expose sensitive information** in logs or error messages

### Performance and Scalability

- **Optimize for performance** without sacrificing readability
- **Consider scalability** in design decisions
- **Use efficient algorithms** and data structures
- **Avoid premature optimization** - focus on correctness first, then optimize if needed

### Naming and Consistency

- **File names and class names must be consistent** with each other
- Use clear, descriptive names that indicate purpose
- Follow Rust naming conventions for this project (snake_case for files/functions, PascalCase for types)

### Testing Requirements

- **Always write test cases** for your code
- **Ensure high test coverage** (aim for >80% coverage)
- Include unit tests, integration tests, and edge case tests
- Test error handling paths, not just happy paths

## Project-Specific Context

This project is JCVM, a Java version manager written in Rust. When making changes:

- Understand the impact on the CLI interface, with utmost care for user experience
- Consider cross-platform compatibility (macOS, Linux, Windows)
- Maintain backward compatibility where possible
- Follow the existing project structure and patterns