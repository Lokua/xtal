# Breakpoint Deserialization and Parameter Handling System

## Overview

This documentation explains the flow of converting a `BreakpointConfig` to a
`Breakpoint`, including how different parameter types are handled. The
documentation is also relevant for converting other `control_script::config`
types however breakpoints are more complex so they cover all the bases.

## Core Data Flow

1. **Initial Deserialization**: YAML is deserialized into a `BreakpointConfig`
   object using `serde_yml`
2. **Config to Breakpoint Conversion**: Each `BreakpointConfig` is converted to
   a `Breakpoint` using the `From<BreakpointConfig>` implementation
3. **Hot Parameter Resolution**: Later in the workflow, any `ParamValue::Hot`
   parameters are resolved using the dependency graph

## Parameter Types

The system handles three types of parameters:

### 1. Cold Parameters (`ParamValue::Cold`)

- **Definition**: Direct numeric values (`f32`) wrapped in `ParamValue::Cold`
- **Example**: `amplitude: ParamValue::Cold(50.0)`
- **Handling**: During conversion, these are unwrapped and directly set on the
  corresponding field in the `Breakpoint`

### 2. Hot Parameters (`ParamValue::Hot`)

- **Definition**: String references to other parameters wrapped in
  `ParamValue::Hot`
- **Example**: `amplitude: ParamValue::Hot("some.other.param")`
- **Handling**:
  - During initial conversion, these are replaced with default values (typically
    0.0)
  - Later resolved separately via the dependency graph using `set_from_param`
    with keypaths

### 3. Non-Param Fields (direct values)

- **Definition**: Direct non-numeric values (e.g., strings) that aren't wrapped
  in `ParamValue`
- **Example**: `easing: "ease_in"`
- **Handling**: Converted to appropriate enum types (e.g., `Easing::from_str`)
  during the conversion process

## Conversion Implementation

The `From<BreakpointConfig>` implementation follows these steps:

1. Create a basic `Breakpoint` with position, value, and a default kind
2. Use reflection to iterate through fields in the config's kind
3. For each field:
   - If it's a `ParamValue::Cold`, extract the `f32` and call `set_field`
   - If it's a `ParamValue::Hot`, skip it (will be handled later)
   - If it's a non-`ParamValue` field, call `set_non_param_field`

```rust
// Example flow for a Random breakpoint
BreakpointConfig {
    position: 0.0,
    value: ParamValue::Cold(100.0),
    kind: Kind::Random {
        amplitude: ParamValue::Cold(50.0),
    },
}
```

Becomes:

```rust
Breakpoint {
    position: 0.0,
    value: 100.0, // Unwrapped from Cold
    kind: Kind::Random {
        amplitude: 50.0, // Unwrapped from Cold
    },
}
```

## Parameter Setting Methods

The system has three specialized methods for parameter setting:

### 1. `set_field(&mut self, name: &str, value: f32)`

- **Purpose**: Sets numeric fields on the breakpoint
- **Used by**: Initial conversion for `ParamValue::Cold` values
- **Implementation**: Matches on the kind variant and field name to set the
  appropriate field

### 2. `set_non_param_field(&mut self, name: &str, value: &dyn Reflect)`

- **Purpose**: Handles non-numeric fields (like strings that need conversion to
  enums)
- **Used by**: Initial conversion for non-`ParamValue` fields
- **Implementation**: Uses reflection and custom conversion logic (e.g.,
  `from_str`)

### 3. `set_from_param(&mut self, name: &str, value: f32)` [From the `SetFromParam` trait]

- **Purpose**: Handles parameter resolution via keypaths
- **Used by**: ONLY for hot parameter resolution after initial conversion
- **Important**: Accepts both direct field names (for backward compatibility)
  and keypaths like "breakpoints.0.amplitude"
- **Implementation**: Extracts the key from the path and delegates to
  `set_field`

## Hot Parameter Resolution Flow

```rust
// Later in the code
fn resolve_breakpoint_params(&self, node_name: &str, breakpoints: &Vec<Breakpoint>) -> Vec<Breakpoint> {
    let mut breakpoints = breakpoints.clone();

    if let Some(params) = self.dep_graph.node(node_name) {
        for (param_name, param_value) in params.iter() {
            // Example param_name: "breakpoints.0.amplitude"
            let value = param_value.cold_or(|name| self.get_raw(&name));
            breakpoints[index].set_from_param(&param_name, value);
        }
    }

    breakpoints
}
```

## Important Notes

1. `set_from_param` is primarily for hot parameter resolution but is backward
   compatible with direct field names
2. The reflection system handles the traversal of nested enum fields
3. Default values (typically 0.0) are used for hot parameters during initial
   conversion
4. Field setting logic is organized by kind variant to make it clear what fields
   each variant supports

This modular approach separates the concerns of:

- Traversing the configuration structure (reflection)
- Setting numeric parameters (set_field)
- Converting non-numeric parameters (set_non_param_field)
- Resolving hot parameters (set_from_param)
