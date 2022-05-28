
    .section .data
    .global img_start
    .global img_end
    .align 12
img_start:
    .incbin "../fat.img"
img_end:
