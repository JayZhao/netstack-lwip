# netstack-lwip

这是一个专门用于将 TUN 接口的网络数据包转换为 TCP 流和 UDP 数据包的网络协议栈。它使用 lwIP 作为底层网络协议栈实现。

## 功能特点

- 基于 lwIP 实现的轻量级网络协议栈
- 支持 TCP 和 UDP 协议
- 异步 I/O 处理（基于 Tokio）
- 高性能的数据包处理
- 支持 IPv4 和 IPv6
- 内存安全（使用 Rust 实现）

## 架构设计

### 数据流设计

项目采用 Rust 的 split 模式设计数据流，这种设计模式将双向数据流分割为单向流，以实现高效的并发处理：

```plaintext
数据流向示意图：

[TUN 设备]          [网络栈]
    ↑                  ↑
tun_sink <---- stack_stream  (网络栈 → TUN 设备)
    ↓                  ↓
tun_stream ---> stack_sink   (TUN 设备 → 网络栈)
```

这种设计有以下优势：
- **并发处理**：支持数据的同时读写
- **所有权分离**：符合 Rust 的所有权规则
- **资源安全**：避免数据竞争
- **灵活性**：读写流可以在不同的异步任务中使用

### 必需的 Trait 实现

TUN 设备需要实现来自 Rust `futures` 库的标准异步 trait。这些 trait 是 Rust 异步编程的基础抽象，被广泛应用于整个 Rust 生态系统：

- **Stream**：异步版本的 `Iterator`，用于按序列产生值
- **Sink**：异步版本的写入接口，用于消费值

```rust
// 从 futures 库导入这些标准 trait
use futures::{Stream, Sink};
use std::pin::Pin;
use std::task::{Context, Poll};

// Stream trait：用于异步数据读取
pub trait Stream {
    // 流中元素的类型
    type Item;

    // 尝试从流中获取下一个值（类似于同步 Iterator 的 next）
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
}

// Sink trait：用于异步数据写入
pub trait Sink<Item> {
    // 错误类型
    type Error;

    // 检查是否准备好接收下一个值
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;

    // 开始发送一个值（类似于同步的 write）
    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error>;

    // 确保所有数据都被发送（类似于同步的 flush）
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;

    // 关闭写入流（类似于同步的 close）
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;
}

// TUN 设备实现示例
impl Stream for TunDevice {
    // 读取的数据是字节数组，可能会有 IO 错误
    type Item = Result<Vec<u8>, io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // 实现从 TUN 设备异步读取数据的逻辑
    }
}

impl Sink<Vec<u8>> for TunDevice {
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // 检查设备是否准备好接收数据
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
        // 开始向设备写入数据
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // 确保数据被完全写入设备
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // 关闭设备的写入端
    }
}
```

这些 trait 在 Rust 异步生态中被广泛使用，例如：
- 网络 IO（tokio::net）
- 文件 IO（tokio::fs）
- 异步通道（tokio::sync::mpsc）
- WebSocket 连接
- 数据库连接
- 等等...

使用这些标准 trait 的好处是：
1. **互操作性**：可以与其他异步 Rust 库无缝集成
2. **标准化**：使用经过验证的异步模式
3. **工具支持**：可以使用 futures 库提供的各种工具和组合器
4. **可组合性**：易于与其他异步组件组合使用

## 使用示例

下面是一个基本的使用示例，展示了如何使用这个网络协议栈：

```rust
// 1. 创建网络栈实例
let (stack, mut tcp_listener, udp_socket) = netstack::NetStack::new();

// 2. 分割网络栈的读写流
let (mut stack_sink, mut stack_stream) = stack.split();
// stack_sink：用于向网络栈写入数据
// stack_stream：用于从网络栈读取数据

// 3. 分割 TUN 设备的读写流
// tun 需要实现 `Stream` 和 `Sink` trait
let (mut tun_sink, mut tun_stream) = tun.split(); 
// tun_sink：用于向 TUN 设备写入数据
// tun_stream：用于从 TUN 设备读取数据

// 4. 处理从网络栈到 TUN 设备的数据流
tokio::spawn(async move {
    while let Some(pkt) = stack_stream.next().await {  // 从网络栈读取
        if let Ok(pkt) = pkt {
            tun_sink.send(pkt).await.unwrap();        // 写入 TUN 设备
        }
    }
});

// 5. 处理从 TUN 设备到网络栈的数据流
tokio::spawn(async move {
    while let Some(pkt) = tun_stream.next().await {   // 从 TUN 设备读取
        if let Ok(pkt) = pkt {
            stack_sink.send(pkt).await.unwrap();      // 写入网络栈
        }
    }
});

// 6. 处理 TCP 连接
// 从网络栈提取 TCP 连接并发送到分发器
tokio::spawn(async move {
    while let Some((stream, local_addr, remote_addr)) = tcp_listener.next().await {
        tokio::spawn(handle_inbound_stream(
            stream,
            local_addr,
            remote_addr,
        ));
    }
});

// 7. 处理 UDP 数据包
// 在网络栈和 NAT 管理器之间收发 UDP 数据包
tokio::spawn(async move {
    handle_inbound_datagram(udp_socket).await;
});
```

## 详细说明

### 组件说明

1. **NetStack**
   - 核心组件，提供网络协议栈功能
   - 通过 `new()` 方法创建实例
   - 返回三个组件：
     - stack：实现了 `Stream` 和 `Sink` trait 的双向数据流
     - tcp_listener：TCP 连接监听器
     - udp_socket：UDP 数据包处理器

2. **Split 模式组件**
   - **Stack Split**（通过 `futures::Stream` 和 `futures::Sink` 实现）
     - stack_sink：接收来自 TUN 的数据并写入网络栈
       ```rust
       impl Sink<Vec<u8>> for StackSink {
           type Error = io::Error;
           // 实现网络栈的写入功能
       }
       ```
     - stack_stream：从网络栈读取数据并准备发送到 TUN
       ```rust
       impl Stream for StackStream {
           type Item = Result<Vec<u8>, io::Error>;
           // 实现网络栈的读取功能
       }
       ```
   - **TUN Split**（需要实现相同的 trait）
     - tun_sink：接收来自网络栈的数据并写入 TUN
     - tun_stream：从 TUN 读取数据并准备发送到网络栈

3. **数据流转换示意图**
   ```plaintext
   [TUN 设备]                    [网络栈]
   (Stream/Sink)               (Stream/Sink)
       ↑                            ↑
   tun_sink <------------ stack_stream
       ↓                            ↓
   tun_stream -----------> stack_sink

   注：两端都实现了 Stream 和 Sink trait，
   使得数据可以在两个方向上异步流动
   ```

4. **TCP 处理**
   - 支持多个 TCP 连接的并发处理
   - 自动管理连接的生命周期
   - 提供连接的本地地址和远程地址信息

5. **UDP 处理**
   - 集成 NAT 功能
   - 支持 UDP 会话管理
   - 处理无连接的数据包转发

### 使用要求

1. **TUN 设备要求**
   - 需要实现 `futures::Stream` trait
     - 用于从设备读取数据包
     - 返回类型应为 `Result<Vec<u8>, io::Error>`
   - 需要实现 `futures::Sink` trait
     - 用于向设备写入数据包
     - 接受类型为 `Vec<u8>`
   - 可以使用现有的 TUN 实现库（如 `tun-tap`、`tokio-tun` 等）

2. **异步运行时**
   - 需要 Tokio 异步运行时环境
   - 所有 I/O 操作都是异步的
   - 支持并发的数据流处理
