//! Frozen DC1 accessor contract; execute only after the source EXEC_SHA is posted.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use rodas5p_core::directed::{Interval, add_up, mul_up, sqrt_up, sub_up};
use rodas5p_integrators::packet_window::{
    PacketComplexInterval, PacketCurveOutput, PacketFrozenOutput, RepresentedPacketCurve,
};
use rodas5p_integrators::{
    CausalWeights, CollectiveModel, PacketWorkCounters, compile_packet, generate_candidate,
};

thread_local! {
    static COUNT_ALLOC: Cell<bool> = const { Cell::new(false) };
    static ALLOC_COUNT: Cell<usize> = const { Cell::new(0) };
}
struct CountingAllocator;
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT_ALLOC.with(|on| {
            if on.get() {
                ALLOC_COUNT.with(|n| n.set(n.get() + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        COUNT_ALLOC.with(|on| {
            if on.get() {
                ALLOC_COUNT.with(|n| n.set(n.get() + 1));
            }
        });
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        COUNT_ALLOC.with(|on| {
            if on.get() {
                ALLOC_COUNT.with(|n| n.set(n.get() + 1));
            }
        });
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[test]
fn dc1_same_query_immutable_weighted_center_displacement() {
    // The old center/radius bits come from the independently retained A01
    // local-certificate payload, arm chi=0, requested time H/2.
    let xi = [
        f64::from_bits(0xbfe8000000000000),
        f64::from_bits(0x3fc0000000000000),
        f64::from_bits(0x3ff0000000000008),
    ];
    let weights = [
        f64::from_bits(0x3fc0000000000000),
        f64::from_bits(0x3fd8000000000000),
        f64::from_bits(0x3fe0000000000800),
    ];
    let initial = [
        (
            f64::from_bits(0x3fd0000000000000),
            f64::from_bits(0xbfc0000000000000),
        ),
        (
            f64::from_bits(0xbfc8000000000000),
            f64::from_bits(0x3fb0000000000000),
        ),
        (
            f64::from_bits(0x3fc0000000000000),
            f64::from_bits(0x3fc8000000000000),
        ),
    ];
    let horizon = f64::from_bits(0x3fa0000000000000);
    let time = f64::from_bits(0x3f90000000000000);
    let model = CollectiveModel::new(
        xi.to_vec(),
        weights.to_vec(),
        initial.to_vec(),
        f64::from_bits(0x3fe8000000000000),
        f64::from_bits(0),
        f64::from_bits(0x3ff4000000000000),
        f64::from_bits(0),
    )
    .unwrap();
    let mut work = PacketWorkCounters::default();
    let packet = compile_packet(&model, horizon, 4, &mut work).unwrap();
    let table = CausalWeights::compile(4, 12);
    let candidate = generate_candidate(&packet, 4, 2, &table, &mut work).unwrap();
    let curve = RepresentedPacketCurve::freeze(&candidate, &packet, &mut work).unwrap();
    curve
        .validate_source(&candidate, &packet, &mut work)
        .unwrap();
    let zero = Interval::point(0.0).unwrap();
    let mut boxes = [PacketComplexInterval { re: zero, im: zero }; 3];
    curve
        .enclose_modes_at_into(time, &mut boxes, &mut work)
        .unwrap();
    let frozen = PacketFrozenOutput::new(curve.clone(), &[0.0; 3], &mut work).unwrap();
    let direct = PacketCurveOutput::new(curve);
    let mut direct_workspace = direct.workspace(&mut work).unwrap();
    let mut frozen_workspace = frozen.workspace(&mut work).unwrap();
    let before = work.curve_output_certificate_ops;
    let view = direct
        .enclose_at(time, &[0.0; 3], &mut direct_workspace, &mut work)
        .unwrap();
    assert_eq!(work.curve_output_certificate_ops - before, 120); // old 110 + 3N+1
    let expected_modes = [
        (0x3fcfdbaf1e822c00, 0xbfc047d743511600),
        (0xbfc802fe4fee0510, 0x3fafdbfdc0d806c0),
        (0x3fbf6f70d86c0000, 0x3fc82f93b8510000),
    ];
    let expected_mode_radii = [0x3dd27320a426b656, 0x3dc54dcc41606711, 0x3dc27321c25e2b73];
    for j in 0..3 {
        assert_eq!(
            (view.modes()[j].0.to_bits(), view.modes()[j].1.to_bits()),
            expected_modes[j]
        );
        assert_eq!(view.mode_radii()[j].to_bits(), expected_mode_radii[j]);
    }
    assert_eq!(
        (view.mean().0.to_bits(), view.mean().1.to_bits()),
        (0x3f96b195df903c40, 0x3fba16dd7ba547dc)
    );
    assert_eq!(view.mean_radius().to_bits(), 0x3dba1783ed6936ad);
    let center = view.modes();
    let mut square = 0.0;
    for j in 0..3 {
        let dr = sub_up(boxes[j].re.hi, center[j].0)
            .unwrap()
            .max(sub_up(center[j].0, boxes[j].re.lo).unwrap());
        let di = sub_up(boxes[j].im.hi, center[j].1)
            .unwrap()
            .max(sub_up(center[j].1, boxes[j].im.lo).unwrap());
        let radius_square = add_up(mul_up(dr, dr).unwrap(), mul_up(di, di).unwrap()).unwrap();
        square = add_up(square, mul_up(weights[j], radius_square).unwrap()).unwrap();
    }
    let box_displacement = sqrt_up(square).unwrap();
    assert!(view.weighted_center_displacement() >= box_displacement);
    let direct_delta = view.weighted_center_displacement();
    let direct_base = view.weighted_radius();
    let before_frozen = work.curve_output_certificate_ops;
    let certified = frozen
        .enclose_at(time, &mut frozen_workspace, &mut work)
        .unwrap();
    assert_eq!(work.curve_output_certificate_ops - before_frozen, 120);
    assert_eq!(
        certified.weighted_center_displacement().to_bits(),
        direct_delta.to_bits()
    );
    assert_eq!(certified.weighted_radius().to_bits(), direct_base.to_bits());
    let before_getters = work;
    ALLOC_COUNT.with(|n| n.set(0));
    COUNT_ALLOC.with(|on| on.set(true));
    let repeated = (
        certified.weighted_center_displacement(),
        certified.weighted_center_displacement(),
    );
    COUNT_ALLOC.with(|on| on.set(false));
    assert_eq!(repeated.0.to_bits(), repeated.1.to_bits());
    assert_eq!(ALLOC_COUNT.with(Cell::get), 0);
    assert_eq!(work, before_getters);
}
