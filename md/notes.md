# Bevy基础渲染 (0.13)

**随便写的前言:** 关于Bevy渲染的资料可谓又少又过时，本人遂分享一下我所知道的关于Bevy渲染的一切。同时这也是我第一次写这种文章，肯定有很多不足，欢迎指正补充！

另外，请注意本文中使用的词语：“只能，必须，一定” 和 “可以”

还有，这里的一章一章不是说这一章的内容就是章标题，这个标题只能算是一个示例吧，是这章的一部分。

## 第一章 自定义材质

首先，材质需要继承/实现
- `Asset`因为材质本身是一种资产
- `AsBindGroup`因为材质的数据是需要绑定给Shader的
- `Material`/`Material2d`因为你要指定这个材质适用的Shader，或者自定义渲染管线，有点像Unity里面把Shader拖动给材质的意思。

### 什么是BindGroup？

我们任意打开一个Shader，例如`bevy_sprite/src/render/sprite.wgsl`:

```rust
#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

#import bevy_render::{
    maths::affine_to_square,
    view::View,
}

// ===================
// 其中，这里的group就是BindGroup的索引，binding就是这个数据在这个BindGroup中的索引
// ===================
@group(0) @binding(0) var<uniform> view: View;

...

// ===================
// 同理，这里也是
// ===================
@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

...
```

### BindGroup可以承载哪些数据？

我们可以通过查看`AsBindGroup`这个过程宏(proc-macro)的实现来得到答案：

```rust
#[proc_macro_derive(
    AsBindGroup,
    attributes(uniform, storage_texture, texture, sampler, bind_group_data, storage)
)]
pub fn derive_as_bind_group(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    as_bind_group::derive_as_bind_group(input).unwrap_or_else(|err| err.to_compile_error().into())
}
```

可以看到它支持`uniform`, `storage_texture`, `texture`, `sampler`, `storage`，这个`bind_group_data`我们暂时不管，因为它不是BindGroup承载的数据，我们待会介绍。

值得注意的是，Wgsl不止支持这些类型，更多类型我们一会会讲到。

#### `uniform`和`storage`

这二者都是缓冲区(buffer)，它们都可以承载例如`i32`之类的primitive type或者你自定义的结构体，并且都可以设置dynamic offset（待会介绍）。区别在于`uniform`一次**只能**承载**一个只读**的东西，比如上面`sprite.wgsl`中的`View`，而`storage`**可以**一次承载**一个或多个(array)只读/只写/读写**的东西。

#### `storage_texture`, `texture`和`sampler`

这三者都与材质有关，两个带texture的都是材质（这不废话嘛），`sampler`是**只能在Fragment阶段使用的**采样器（待会详细介绍）。两个材质的区别在于，`storage_texture`可以读写，不能使用`sampler`采样，而`texture`只读，**可以**使用`sampler`采样。

### 如何使用 `AsBindGroup`？

其实`StandardMaterial`已经覆盖几乎所有用法了。基本上就是在对应类型的数据上面加对应的attr。

| Attr                 | Wgsl示例                                                 | Rust类型                                                                                                       | 含义                                                                                  | 额外说明                                                                         |
| -------------------- | -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `uniform(x)`         | `var<uniform> data: TheTypeOfx`                          | 任意实现了`ShaderType`的数据                                                                                   | 将这个数据绑定到`binding(x)`                                                          | 这个在`StandardMaterial`里面没有用到，可以看`examples/shader/shader_material.rs` |
| `uniform(x, Type)`   | `var<uniform> data: Type`                                | 任意实现了`ShaderType`的数据                                                                                   | 将这个材质里剩下的数据打包成`Type`并绑定到`binding(x)`                                | 这个就是`StandardMaterial`里面的用法                                             |
| `texture(x)`         | `var texture: texture_2d<f32>`                           | `Handle<Image>`或者`Option<Handle<Image>>`                                                                     | 绑个材质到`binding(x)`                                                                | 无                                                                               |
| `storage_texture(x)` | `var texture: texture_storage_2d<rgba8unorm, read_only>` | 同`texture`                                                                                                    | 以只读的方式绑个`rgba8unorm`格式（Bevy默认的格式）的`storage_texture`，到`binding(x)` | 无                                                                               |
| `sampler(x)`         | `var texture_sampler: sampler`                           | 没有对应的Rust类型，因为这个attr要加在你要使用这个采样器的材质的那个成员变量上面（表达的不太好建议直接看例子） | 绑个采样器到`binding(x)`                                                              | 无                                                                               |

完整示例：

```rust
#[derive(Asset, AsBindGroup, TypePath)]
struct MyMaterial {
    #[uniform(0)]
    pub color: Color,

    #[texture(
        1,
        dimension = "2d",
        sample_type = "float",
        multisampled = true,
        filterable = true
    )]
    #[sampler(2, sampler_type = "filtering")]
    pub texture: Option<Handle<Image>>,

    #[storage_texture(3, dimension = "2d")]
    pub storage_texture: Handle<Image>,

    #[storage(4, read_only, buffer)]
    pub buffer: Buffer,
}
```

可以看到除了表格里面列的，还有很多额外的设置，这些我们之后会讲。如果你对这些attr的意思了解，那么你完全可以随便写个错误的attr：`#[texture(0, aaa)]`，Bevy的文档做的很全面，所以它会报错并且提示你哪些是可用的。

## 第二章 后处理

这一章内容可能会有点多，*请坐稳扶好*。

Shader部分随便写一个吧，不是重点
