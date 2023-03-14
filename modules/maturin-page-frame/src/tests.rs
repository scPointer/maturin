use super::*;

#[test]
fn test_default_config() {
    struct DefaultConfig;
    impl PageFrameConfig for DefaultConfig {}
    type MyFrame = Frame<DefaultConfig>;
    const PAGE_SIZE: usize = 0x1000;

    println!("testing default config");

    let range = vec![0..0x2000, 0x5000..0x8000];
    let frame_id = vec![0, 1, 5, 6, 7];
    let mut frames: Vec<MyFrame> = vec![];
    unsafe {
        MyFrame::init(range);
    }
    for i in 0..5 {
        let frame = MyFrame::new().unwrap();
        assert_eq!(frame.start_paddr(), frame_id[i] * PAGE_SIZE);
        assert_eq!(frame.size(), PAGE_SIZE);
        frames.push(frame);
    }
    assert!(MyFrame::new().is_none());
    frames.clear();
    let frame = MyFrame::new().unwrap();
    assert!(frame.start_paddr() < 0x8000);
}
