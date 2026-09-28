/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

/*
 This file incorporates work covered by the following copyright and
 permission notice:

    https://github.com/rust-skia/rust-skia/blob/master/skia-safe/examples/vulkan-window (three files, modified)

    MIT License

    Copyright (c) 2019 LongYinan & Armin Sander
    Copyright (c) 2019 rust-skia Contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.
*/

use log::{debug, error, info};
use std::{error::Error, sync::Arc};
use vulkano::{
    VulkanLibrary,
    device::{
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
        physical::PhysicalDeviceType,
    },
    format,
    instance::{
        Instance, InstanceCreateFlags, InstanceCreateInfo, InstanceExtensions,
        debug::{
            DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessengerCallback,
            DebugUtilsMessengerCreateInfo,
        },
    },
    swapchain::{self, Surface},
};

use crate::{
    events::EventHandling,
    wnd::{CustomWinitEvent, EventLoopAwaker, WindowBackend, WinitElAwaker},
};

use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use ash::vk::Handle;
use std::ptr;
use vulkano::{
    Validated, VulkanError, VulkanObject,
    image::{Image, ImageUsage},
    swapchain::{
        PresentMode, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo, SwapchainPresentInfo,
        acquire_next_image,
    },
    sync::{self, GpuFuture},
};

use skia_safe::{
    ColorType,
    gpu::{self, backend_render_targets, direct_contexts, surfaces, vk},
};

pub struct VulkanRenderContext {
    queue: Option<Arc<Queue>>,
}

impl VulkanRenderContext {
    pub fn new() -> Self {
        Self { queue: None }
    }

    pub fn renderer_for_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: Arc<Window>,
    ) -> Option<VulkanRenderer> {
        // lazily set up a shared instance, device, and queue to use for all subsequent renderers
        let validate = false;
        let queue = self
            .queue
            .get_or_insert_with(|| Self::shared_queue(event_loop, window.clone(), validate));

        VulkanRenderer::new(window.clone(), queue.clone())
    }

    fn shared_queue(
        event_loop: &ActiveEventLoop,
        window: Arc<Window>,
        validate: bool,
    ) -> Arc<Queue> {
        let library = VulkanLibrary::new().expect("Vulkan libraries not found on system");

        // The first step of any Vulkan program is to create an instance.
        //
        // When we create an instance, we have to pass a list of extensions that we want to enable.
        //
        // All the window-drawing functionalities are part of non-core extensions that we need to
        // enable manually. To do so, we ask `Surface` for the list of extensions required to draw
        // to a window.
        let required_extensions = Surface::required_extensions(event_loop).unwrap();
        let (enabled_extensions, enabled_layers, debug_utils_messengers) =
            Self::validation_instance_config(&library, required_extensions, validate);

        // Now creating the instance.
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                // Enable enumerating devices that use non-conformant Vulkan implementations.
                // (e.g. MoltenVK)
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions,
                enabled_layers,
                debug_utils_messengers,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| {
            panic!("Could not create instance supporting: {required_extensions:?}")
        });

        // Choose device extensions that we're going to use. In order to present images to a
        // surface, we need a `Swapchain`, which is provided by the `khr_swapchain` extension.
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        // In order to select the proper queue family we need a reference to the window's surface
        // so we can check whether the queue supports it. Note that in a future vulkano release
        // this requirement will go away once it can check for `presentation_support` from the
        // event_loop's display (see commented usage below…)
        let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();

        // We then choose which physical device to use. First, we enumerate all the available
        // physical devices, then apply filters to narrow them down to those that can support our
        // needs.
        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .unwrap()
            .filter(|p| {
                // Some devices may not support the extensions or features that your application,
                // or report properties and limits that are not sufficient for your application.
                // These should be filtered out here.
                p.supported_extensions().contains(&device_extensions)
            })
            .filter_map(|p| {
                // For each physical device, we try to find a suitable queue family that will
                // execute our draw commands.
                //
                // Devices can provide multiple queues to run commands in parallel (for example a
                // draw queue and a compute queue), similar to CPU threads. This is
                // something you have to have to manage manually in Vulkan. Queues
                // of the same type belong to the same queue family.
                //
                // Here, we look for a single queue family that is suitable for our purposes. In a
                // real-world application, you may want to use a separate dedicated transfer queue
                // to handle data transfers in parallel with graphics operations.
                // You may also need a separate queue for compute operations, if
                // your application uses those.
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        // We select a queue family that supports graphics operations. When drawing
                        // to a window surface, as we do in this example, we also need to check
                        // that queues in this queue family are capable of presenting images to the
                        // surface.
                        q.queue_flags.intersects(QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                        //  && p.presentation_support(_i as u32, event_loop).unwrap() // unreleased
                    })
                    // The code here searches for the first queue family that is suitable. If none
                    // is found, `None` is returned to `filter_map`, which
                    // disqualifies this physical device.
                    .map(|i| (p, i as u32))
            })
            // All the physical devices that pass the filters above are suitable for the
            // application. However, not every device is equal, some are preferred over others.
            // Now, we assign each physical device a score, and pick the device with the lowest
            // ("best") score.
            //
            // In this example, we simply select the best-scoring device to use in the application.
            // In a real-world setting, you may want to use the best-scoring device only as a
            // "default" or "recommended" device, and let the user choose the device themself.
            .min_by_key(|(p, _)| {
                // We assign a lower score to device types that are likely to be faster/better.
                match p.properties().device_type {
                    PhysicalDeviceType::DiscreteGpu => 0,
                    PhysicalDeviceType::IntegratedGpu => 1,
                    PhysicalDeviceType::VirtualGpu => 2,
                    PhysicalDeviceType::Cpu => 3,
                    PhysicalDeviceType::Other => 4,
                    _ => 5,
                }
            })
            .expect("No suitable physical device found");

        // Print out the device we selected
        debug!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        // Now initializing the device. This is probably the most important object of Vulkan.
        //
        // An iterator of created queues is returned by the function alongside the device. Each
        // queue has a reference to its instance so we don't need to store that directly.
        let (_, mut queues) = Device::new(
            // Which physical device to connect to.
            physical_device,
            DeviceCreateInfo {
                // A list of optional features and extensions that our program needs to work
                // correctly. Some parts of the Vulkan specs are optional and must be enabled
                // manually at device creation. In this example the only thing we are going to need
                // is the `khr_swapchain` extension that allows us to draw to a window.
                enabled_extensions: device_extensions,

                // The list of queues that we are going to use. Here we only use one queue, from
                // the previously chosen queue family.
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],

                ..Default::default()
            },
        )
        .expect("Device initialization failed");

        // Since we can request multiple queues, the `queues` variable is in fact an iterator. We
        // only use one queue in this example, so we just retrieve the first and only element of
        // the iterator.
        queues.next().unwrap()
    }

    fn validation_instance_config(
        library: &Arc<VulkanLibrary>,
        required_extensions: InstanceExtensions,
        validate: bool,
    ) -> (
        InstanceExtensions,
        Vec<String>,
        Vec<DebugUtilsMessengerCreateInfo>,
    ) {
        if !validate {
            return (required_extensions, Vec::new(), Vec::new());
        }

        const VALIDATION_LAYER: &str = "VK_LAYER_KHRONOS_validation";

        let mut enabled_extensions = required_extensions;
        let mut enabled_layers = Vec::new();
        let mut debug_utils_messengers = Vec::new();

        let has_validation_layer = library
            .layer_properties()
            .map(|layers| {
                layers
                    .into_iter()
                    .any(|layer| layer.name() == VALIDATION_LAYER)
            })
            .unwrap_or(false);

        if has_validation_layer {
            enabled_layers.push(VALIDATION_LAYER.to_owned());

            if library.supported_extensions().ext_debug_utils {
                enabled_extensions.ext_debug_utils = true;
                debug_utils_messengers.push(DebugUtilsMessengerCreateInfo {
                    message_severity: DebugUtilsMessageSeverity::ERROR
                        | DebugUtilsMessageSeverity::WARNING
                        | DebugUtilsMessageSeverity::INFO
                        | DebugUtilsMessageSeverity::VERBOSE,
                    message_type: DebugUtilsMessageType::GENERAL
                        | DebugUtilsMessageType::VALIDATION
                        | DebugUtilsMessageType::PERFORMANCE,
                    ..DebugUtilsMessengerCreateInfo::user_callback(unsafe {
                        DebugUtilsMessengerCallback::new(
                            |message_severity, message_type, callback_data| {
                                info!(
                                    "[vulkan {:?} {:?}] {}",
                                    message_severity, message_type, callback_data.message
                                );
                            },
                        )
                    })
                });
                debug!("Vulkan validation enabled for vulkan-window example");
            } else {
                error!("Vulkan validation requested, but VK_EXT_debug_utils is not available");
            }
        } else {
            error!(
                "Vulkan validation requested, but '{}' is not available on this system",
                VALIDATION_LAYER,
            );
        }

        (enabled_extensions, enabled_layers, debug_utils_messengers)
    }
}

pub struct VulkanRenderer {
    queue: Arc<Queue>,
    images: Vec<Arc<Image>>,
    last_render: Option<Box<dyn GpuFuture>>,

    // Keep `skia_ctx` before `swapchain`: struct fields are dropped in declaration order, and
    // the context must be dropped before the swapchain it renders to.
    skia_ctx: gpu::DirectContext,
    swapchain: Arc<Swapchain>,
    pub window: Arc<Window>,

    swapchain_is_valid: bool,
}

impl VulkanRenderer {
    pub fn select_format(
        formats: Vec<(format::Format, swapchain::ColorSpace)>,
    ) -> Option<format::Format> {
        debug!("{} formats supported", formats.len());
        let preferred_formats = [
            format::Format::R8G8B8A8_SRGB,
            format::Format::B8G8R8A8_UNORM,
            format::Format::R8G8B8A8_UNORM,
        ];
        for p in &preferred_formats {
            for (f, _) in &formats {
                if *f == *p {
                    return Some(*p);
                }
            }
        }
        None
    }

    pub fn new(window: Arc<Window>, queue: Arc<Queue>) -> Option<Self> {
        // Extract references to key structs from the queue
        let device = queue.device();
        let instance = device.instance();
        let library = instance.library();
        let backend_max_api_version = {
            let api_version = device.api_version();
            debug!("Vulkan API version {}", api_version);
            (
                api_version.major as usize,
                api_version.minor as usize,
                api_version.patch as usize,
            )
        };

        // Before we can render to a window, we must first create a `vulkano::swapchain::Surface`
        // object from it, which represents the drawable surface of a window. For that we must wrap
        // the `winit::window::Window` in an `Arc`.
        let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();
        let window_size = window.inner_size();

        // Before we can draw on the surface, we have to create what is called a swapchain.
        // Creating a swapchain allocates the color buffers that will contain the image that will
        // ultimately be visible on the screen. These images are returned alongside the swapchain.
        let (swapchain, _images) = {
            // Querying the capabilities of the surface. When we create the swapchain we can only
            // pass values that are allowed by the capabilities.
            let surface_capabilities = device
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .unwrap();

            // Choosing the internal format that the images will have.
            let formats = device
                .physical_device()
                .surface_formats(&surface, Default::default())
                .unwrap();
            let image_format = match Self::select_format(formats) {
                Some(x) => {
                    debug!("Selected format: {:?}", x);
                    x
                }
                None => {
                    error!("failed to select format");
                    return None;
                }
            };

            // Please take a look at the docs for the meaning of the parameters we didn't mention.
            Swapchain::new(
                device.clone(),
                surface,
                SwapchainCreateInfo {
                    // Some drivers report an `min_image_count` of 1, but fullscreen mode requires
                    // at least 2. Therefore we must ensure the count is at least 2, otherwise the
                    // program would crash when entering fullscreen mode on those drivers.
                    min_image_count: surface_capabilities.min_image_count.max(2),

                    // The size of the window, only used to initially setup the swapchain.
                    //
                    // NOTE:
                    // On some drivers the swapchain extent is specified by
                    // `surface_capabilities.current_extent` and the swapchain size must use this
                    // extent. This extent is always the same as the window size.
                    //
                    // However, other drivers don't specify a value, i.e.
                    // `surface_capabilities.current_extent` is `None`. These drivers will allow
                    // anything, but the only sensible value is the window size.
                    //
                    // Both of these cases need the swapchain to use the window size, so we just
                    // use that.
                    image_extent: window_size.into(),

                    image_usage: ImageUsage::COLOR_ATTACHMENT,

                    image_format,

                    // The present_mode affects what is commonly known as "vertical sync" or "vsync" for short.
                    // The `Immediate` mode is equivalent to disabling vertical sync, while the others enable
                    // vertical sync in various forms. An important aspect of the present modes is their potential
                    // *latency*: the time between when an image is presented, and when it actually appears on
                    // the display.
                    //
                    // Only `Fifo` is guaranteed to be supported on every device. For the others, you must call
                    // [`surface_present_modes`] to see if they are supported.
                    present_mode: PresentMode::Fifo,

                    // The alpha mode indicates how the alpha value of the final image will behave.
                    // For example, you can choose whether the window will be
                    // opaque or transparent.
                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .into_iter()
                        .next()
                        .unwrap(),

                    ..Default::default()
                },
            )
            .unwrap()
        };

        // Swapchain images are wrapped directly into Skia backend render targets during draw.
        // We'll wait until the first `prepare_swapchain` call to populate the image list.
        let images = Vec::new();

        // In some situations, the swapchain will become invalid by itself. This includes for
        // example when the window is resized (as the images of the swapchain will no longer match
        // the window's) or, on Android, when the application went to the background and goes back
        // to the foreground.
        //
        // In this situation, acquiring a swapchain image or presenting it will return an error.
        // Rendering to an image of that swapchain will not produce any error, but may or may not
        // work. To continue rendering, we need to recreate the swapchain by creating a new
        // swapchain. Here, we remember that we need to do this for the next loop iteration.
        //
        // Since we haven't populated per-image metadata yet, we'll start in an invalid state to
        // flag that swapchain-dependent state needs to be recreated before we render.
        let swapchain_is_valid = false;

        // In the `draw_and_present` method below we are going to submit commands to the GPU.
        // Submitting a command produces an object that implements the `GpuFuture` trait, which
        // holds the resources for as long as they are in use by the GPU.
        //
        // Destroying the `GpuFuture` blocks until the GPU is finished executing it. In order to
        // avoid that, we store the submission of the previous frame here.
        let last_render = Some(sync::now(device.clone()).boxed());

        // Next we need to connect Skia's gpu backend to the device & queue we've set up.
        let skia_ctx = unsafe {
            // In order to access the vulkan api, we need to give skia some lookup routines
            // to find the expected function pointers for our configured instance & device.
            let get_proc = |gpo| {
                let get_device_proc_addr = instance.fns().v1_0.get_device_proc_addr;

                match gpo {
                    vk::GetProcOf::Instance(instance, name) => {
                        let vk_instance = ash::vk::Instance::from_raw(instance as _);
                        library.get_instance_proc_addr(vk_instance, name)
                    }
                    vk::GetProcOf::Device(device, name) => {
                        let vk_device = ash::vk::Device::from_raw(device as _);
                        get_device_proc_addr(vk_device, name)
                    }
                }
                .map(|f| f as _)
                .unwrap_or_else(|| {
                    error!("Vulkan: failed to resolve {}", gpo.name().to_str().unwrap());
                    ptr::null()
                })
            };

            // We then pass skia_safe references to the whole shebang, resulting in a DirectContext
            // from which we'll be able to get a canvas reference that draws directly to swapchain images
            // on the swapchain.
            direct_contexts::make_vulkan(
                &vk::BackendContext::new_builder(
                    instance.handle().as_raw() as _,
                    device.physical_device().handle().as_raw() as _,
                    device.handle().as_raw() as _,
                    (
                        queue.handle().as_raw() as _,
                        queue.queue_family_index() as usize,
                    ),
                    &get_proc,
                    Some(backend_max_api_version.into()),
                )
                .build(),
                None,
            )
            .unwrap()
        };

        Some(VulkanRenderer {
            queue,
            images,
            last_render,
            skia_ctx,
            swapchain,
            window,
            swapchain_is_valid,
        })
    }

    pub fn invalidate_swapchain(&mut self) {
        // Typically called when the window size changes and we need to recreate swapchain resources.
        self.swapchain_is_valid = false;
    }

    pub fn prepare_swapchain(&mut self) {
        // It is important to call this function from time to time, otherwise resources
        // will keep accumulating and you will eventually reach an out of memory error.
        // Calling this function polls various fences in order to determine what the GPU
        // has already processed, and frees the resources that are no longer needed.
        if let Some(last_render) = self.last_render.as_mut() {
            last_render.cleanup_finished();
        }

        // Whenever the window resizes we need to recreate everything dependent on the
        // window size. In this example that includes the swapchain and image metadata.
        let window_size: PhysicalSize<u32> = self.window.inner_size();
        if window_size.width > 0 && window_size.height > 0 && !self.swapchain_is_valid {
            // Use the new dimensions of the window.
            let (new_swapchain, new_images) = self
                .swapchain
                .recreate(SwapchainCreateInfo {
                    image_extent: window_size.into(),
                    ..self.swapchain.create_info()
                })
                .expect("failed to recreate swapchain");

            self.swapchain = new_swapchain;

            self.images = new_images.to_vec();

            self.swapchain_is_valid = true;
        }
    }

    fn get_next_frame(&mut self) -> Option<(u32, SwapchainAcquireFuture)> {
        // prepare to render by identifying the next swapchain image to draw to and acquiring the
        // GpuFuture that we'll be replacing `last_render` with once we submit the frame
        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(self.swapchain.clone(), None).map_err(Validated::unwrap) {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    self.swapchain_is_valid = false;
                    return None;
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };

        // `acquire_next_image` can be successful, but suboptimal. This means that the
        // swapchain image will still work, but it may not display correctly. With some
        // drivers this can be when the window resizes, but it may not cause the swapchain
        // to become out of date.
        if suboptimal {
            self.swapchain_is_valid = false;

            // Consume the acquire future without presenting this stale frame, then let the
            // caller recreate the swapchain and render again.
            // If this is omitted, the stale acquire work gets dropped instead of being chained
            // into `last_render`, which can show up as resize flicker or intermittent stalls.
            self.chain_acquire_without_present(acquire_future);
            return None;
        }

        // Always consume successful acquires in the frame submission chain.
        Some((image_index, acquire_future))
    }

    fn chain_acquire_without_present(&mut self, acquire_future: SwapchainAcquireFuture) {
        self.last_render = Some(
            self.last_render
                .take()
                .unwrap_or_else(|| sync::now(self.queue.device().clone()).boxed())
                .join(acquire_future)
                .boxed(),
        );
    }

    pub fn draw_and_present<F>(&mut self, f: F)
    where
        F: FnOnce(&skia_safe::Canvas, LogicalSize<f32>),
    {
        // find the next swapchain image to render into and acquire a new GpuFuture to block on
        let next_frame = self.get_next_frame().or_else(|| {
            // if suboptimal or out-of-date, recreate the swapchain and try once more
            self.prepare_swapchain();
            self.get_next_frame()
        });

        if let Some((image_index, acquire_future)) = next_frame {
            // pull the appropriate image from the swapchain and attach a skia Surface to it
            let image = self.images[image_index as usize].clone();
            let mut surface = surface_for_image(&mut self.skia_ctx, image);
            let canvas = surface.canvas();

            // use the display's DPI to convert the window size to logical coords and pre-scale the
            // canvas's matrix to match
            let extent: PhysicalSize<u32> = self.window.inner_size();
            let size: LogicalSize<f32> = extent.to_logical(self.window.scale_factor());

            // TODO handle scaling
            /*let scale = (
                (f64::from(extent.width) / size.width as f64) as f32,
                (f64::from(extent.height) / size.height as f64) as f32,
            );
            canvas.reset_matrix();
            canvas.scale(scale);*/

            // pass the surface's canvas and canvas size to the user-provided callback
            f(canvas, size);

            // Flush the surface and explicitly set PRESENT_SRC_KHR for the swapchain image.
            let flush_info = gpu::FlushInfo::default();
            let present_state = gpu::vk::mutable_texture_states::new_vulkan(
                vk::ImageLayout::PRESENT_SRC_KHR,
                self.queue.queue_family_index(),
            );
            self.skia_ctx.flush_surface_with_texture_state(
                &mut surface,
                &flush_info,
                Some(&present_state),
            );
            // Keep this synchronized so the transition is complete before vkQueuePresentKHR.
            self.skia_ctx.submit(gpu::SubmitInfo {
                sync: gpu::SyncCpu::Yes,
                ..gpu::SubmitInfo::default()
            });

            // submit work for this image to the GPU and present it on screen
            self.last_render = self
                .last_render
                .take()
                .unwrap()
                .join(acquire_future)
                .then_swapchain_present(
                    self.queue.clone(),
                    SwapchainPresentInfo::swapchain_image_index(
                        self.swapchain.clone(),
                        image_index,
                    ),
                )
                .then_signal_fence_and_flush()
                .map(|f| Box::new(f) as _)
                .ok();
        }
    }
}

// Create a skia `Surface` (and its associated `.canvas()`) whose render target is the specified image.
fn surface_for_image(skia_ctx: &mut gpu::DirectContext, image: Arc<Image>) -> skia_safe::Surface {
    let [width, height, _] = image.extent();
    let image_object = image.handle().as_raw();

    let format = image.format();

    let (vk_format, color_type) = match format {
        format::Format::B8G8R8A8_UNORM => (
            skia_safe::gpu::vk::Format::B8G8R8A8_UNORM,
            ColorType::BGRA8888,
        ),
        format::Format::R8G8B8A8_UNORM => (
            skia_safe::gpu::vk::Format::R8G8B8A8_UNORM,
            ColorType::RGBA8888,
        ),
        format::Format::R8G8B8A8_SRGB => (
            skia_safe::gpu::vk::Format::R8G8B8A8_SRGB,
            ColorType::SRGBA8888,
        ),
        x => {
            panic!("Unsupported color format {:?}", x);
        }
    };

    let alloc = vk::Alloc::default();
    let image_info = &unsafe {
        vk::ImageInfo::new(
            image_object as _,
            alloc,
            vk::ImageTiling::OPTIMAL,
            vk::ImageLayout::UNDEFINED,
            vk_format,
            1,
            None,
            None,
            None,
            None,
        )
    };

    let render_target = &backend_render_targets::make_vk(
        (width.try_into().unwrap(), height.try_into().unwrap()),
        image_info,
    );

    surfaces::wrap_backend_render_target(
        skia_ctx,
        render_target,
        gpu::SurfaceOrigin::TopLeft,
        color_type,
        None,
        None,
    )
    .unwrap()
}

struct App<'a> {
    render_ctx: VulkanRenderContext, // the shared vulkan device, queue, etc.
    renderer: Option<VulkanRenderer>, // the window-specific skia <-> vulkan bridge
    gel: EventHandling<'a>,
    title: String,
}

impl<'a> App<'a> {
    fn new(title: String) -> Self {
        App {
            render_ctx: VulkanRenderContext::new(),
            title,
            renderer: None,
            gel: EventHandling::new(),
        }
    }
}

impl<'a> ApplicationHandler<CustomWinitEvent> for App<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // since the renderer needs to hold onto a reference to the window, we wrap it in an Arc
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title(&self.title))
                .unwrap(),
        );

        // in this example we only have a single window, but you could also keep a list of
        // VulkanRenderer instances to manage multiple windows
        self.renderer = self
            .render_ctx
            .renderer_for_window(event_loop, window.clone());
        if self.renderer.is_none() {
            panic!("failed to create renderer");
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: CustomWinitEvent) {
        match event {
            CustomWinitEvent::Redraw => self.renderer.as_ref().unwrap().window.request_redraw(),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let mut draw_frame = false;
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.gel.handle_resize(size.width, size.height);
                if let Some(renderer) = self.renderer.as_mut() {
                    // When the window size changes, swapchain-dependent resources need to be
                    // recreated before redrawing the window contents.
                    renderer.invalidate_swapchain();
                    renderer.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                draw_frame = true;
            }
            x => self.gel.handle_winit_event(x),
        }

        if self.gel.wants_redraw()
            && let Some(renderer) = self.renderer.as_mut()
        {
            renderer.window.request_redraw();
        }

        if draw_frame && let Some(renderer) = self.renderer.as_mut() {
            // The swapchain (which manages presentable images and update timing) needs
            // to be cleaned up/validated in between redraws.
            renderer.prepare_swapchain();

            // After the draw routine completes, the contents of the canvas will be displayed
            renderer.draw_and_present(|canvas, _| {
                self.gel.render(canvas);
            });
        }
    }
}

pub struct VkWindowBackend<'a> {
    el: EventLoop<CustomWinitEvent>,
    app: App<'a>,
}

impl<'a> VkWindowBackend<'a> {
    pub fn new(title: String) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            app: App::new(title),
            el: EventLoop::with_user_event().build()?,
        })
    }
}

impl<'a> WindowBackend<'a> for VkWindowBackend<'a> {
    fn run(mut self: Box<Self>) {
        self.el.run_app(&mut self.app).unwrap();
    }

    fn get_size(&self) -> (u32, u32) {
        (500, 500)
    }

    fn use_events(&mut self, events: crate::events::EventHandling<'a>) {
        self.app.gel = events;
    }

    fn get_el_awaker(&self) -> Arc<dyn Send + Sync + EventLoopAwaker> {
        Arc::new(WinitElAwaker {
            proxy: self.el.create_proxy(),
        })
    }
}

pub fn create_vk_window(title: String) -> Result<Box<dyn WindowBackend<'static>>, Box<dyn Error>> {
    Ok(Box::new(VkWindowBackend::new(title)?))
}
