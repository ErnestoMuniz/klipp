# GPUI Animation

`gpui-animation` is a lightweight, fluent animation wrapper for the [GPUI](https://github.com/zed-industries/zed) framework. It aims to simplify the creation of smooth, state-driven transitions and animations on standard GPUI elements with minimal boilerplate.

> [!WARNING]
>
> This crate is currently in its **early development stage**. The API is subject to change.

## ✨ Features

- **Fluent API**: Transform any compatible GPUI element into an animated one using `.with_transition()`.
- **Zero-Copy Interpolation**: High-performance "in-place" style updates to minimize memory cloning during animation frames.
- **Smart Transitions**: Automatic shortest-path interpolation for HSLA colors (no more hue-jumping!) and support for complex types like Gradients and Sizes.
- **Composable**: `AnimatedWrapper` implements standard GPUI traits (`Styled`, `ParentElement`, etc.), so you can keep using the GPUI methods you already know.
- **Intelligent Rollback (Smart Fallback)**:

  **Context-Aware Resumption**: If a high-priority animation (e.g., a Click) interrupts a persistent state (e.g., a Hover), the system "remembers" the background state and restores it seamlessly once the interruption finishes, preventing jarring jumps to the default style.

- **Resource Efficiency (Zero-Idle)**:

  **Async Task Parking**: The background animation tick is strictly event-driven. When no animations are active, the task enters a dormant state using async channel synchronization (`recv().await`).

  **Zero CPU Overhead**: The thread consumes zero CPU cycles while idle and only wakes up instantly when a new animation is registered, ensuring maximum performance for the rest of your application.

## 🚀 Getting Started

Any element that implements `IntoElement + StatefulInteractiveElement + ParentElement + FluentBuilder + Styled` can be wrapped.

### Basic Usage

```rust
fn render(cx: &mut WindowContext) -> impl IntoElement {
        div()
            .id("my-animated-box")
            // Initialize the animation wrapper with a unique ID
            .with_transition("my-animated-box")
            .size_32()
            .bg(rgb(0x2e2e2e))
            // Define a hover transition
            .transition_on_hover(
                std::time::Duration::from_millis(300),
                gpui_animation::transition::general::Linear,
                |hovered, style| {
                    if *hovered {
                        style.bg(rgb(0xff0000)).size_64()
                    } else {
                        style.bg(rgb(0x2e2e2e)).size_32()
                    }
                },
            )
}
```



## 🛠 Supported Properties

| **Category** | **Supported Styles**                                         |
| ------------ | ------------------------------------------------------------ |
| **Colors**   | Background (`Solid`, `LinearGradient`), Border Color, Text Color |
| **Layout**   | Size (Width, Height), Min/Max Size, Margin, Padding          |
| **Visual**   | Opacity, Corner Radii (Border Radius), Box Shadows           |
| **Font**     | FontSize, FontWeight                                         |

## 📖 API Reference

### Initialization

- `.with_transition(id)`: Wraps the element. Requires a unique `ElementId` to track animation state across frames.

### Event-Driven Transitions

These methods automatically trigger the animation cycle when the event occurs:

- `.transition_on_click(duration, transition, modifier)`
- `.transition_on_hover(duration, transition, modifier)`

### Declarative Transitions

Used for reactive state changes:

- `.transition_when(condition, duration, transition, modifier)`
- `.transition_when_some(option, ...)` / `.transition_when_none(...)`

### Priority-Aware Transitions

These variants allow you to explicitly define the precedence of a transition to resolve state conflicts (e.g., ensuring a "Click" animation isn't overridden by a "Hover" state):

- `.transition_on_click_with_priority(duration, transition, priority, modifier)`
- `.transition_on_hover_with_priority(duration, transition, priority, modifier)`
- `.transition_when_with_priority(condition, duration, transition, priority, modifier)`
- `.transition_when_else_with_priority(condition, duration, transition, priority, then, else)`
- `.transition_when_some_with_priority(option, ..., priority, ...)`
- `.transition_when_none_with_priority(option, ..., priority, ...)`

> **AnimationPriority Levels**: `Lowest` , `Low`, `Medium`, `High`, `Realtime`. Transitions with higher priority will override active animations with lower or equal priority.

> [!IMPORTANT]
>
> **Note on Declarative Styling:** > Changes made via `.transition_when()` and its variants do not automatically proactive-propagate the `App` context. Unlike event-based listeners that manage the context internally, you may need to manually invoke a refresh (e.g., `cx.notify()` or `cx.refresh()`) to start the transition when external state changes.

## 🎨 Custom Animation Algorithms

You are not limited to built-in transitions. You can create your own animation curves (Easing functions) by implementing the `Transition` trait.

### 1. Implement the Trait

Only the `calculate` method is required. It maps the linear time progress ($t \in [0, 1]$) to your desired easing value.

```rust
use gpui_animation::transition::Transition;

pub struct MyCustomBounce;

impl Transition for MyCustomBounce {
    fn calculate(&self, t: f32) -> f32 {
        // Example: A simple square curve
        t * t
    }
}
```

### 2. Use it in your UI

Since `Transition` is implemented for `Arc<T>`, and we provide `IntoArcTransition` helpers, you can pass your struct directly:

```rust
div()
		.id("box-1")
    .with_transition("box-1")
    .transition_on_hover(
        Duration::from_millis(500),
        MyCustomBounce, // Your custom algorithm
        |hovered, style| {
            if *hovered { style.mt_10() } else { style.mt_0() }
        }
    )
		.mt_0()
```



## ⚡ Performance

This crate is optimized for high-frequency updates (60/120 FPS):

- **ShadowBackground**: Uses `#[repr(C)]` memory layouts to interpolate private GPUI fields without overhead.
- **FastInterpolatable**: Employs an in-place update strategy to avoid full `StyleRefinement` cloning on every frame.

## 🤝 Contributing

Contributions are welcome! If you find a bug or have a suggestion for new interpolation support (like more Layout properties), please feel free to open an issue or submit a pull request.