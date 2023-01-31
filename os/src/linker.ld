/*
链接脚本语法介绍：https://web.mit.edu/rhel-doc/3/rhel-ld-en-3/scripts.html
risc v session:
.text is a read-only section containing executable code
.data is a read-write section containing global or static variables
.rodata is a read-only section containing const variables
.bss is a read-write section containing uninitialized data
reference: https://michaeljclark.github.io/asm.html
           https://shakti.org.in/docs/risc-v-asm-manual.pdf
*/

OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;

SECTIONS {
    . = BASE_ADDRESS;
    skernel = .;

    stext = .;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    . = ALIGN(4k);
    etext = .;

    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(4k);
    erodata = .;

    sdata = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    . = ALIGN(4k);
    edata = .;

    .bss : {
        *(.bss.stack)
        sbss = .;   /*bss从stack后开始*/
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
    . = ALIGN(4k);
    ebss = .;
    ekernel = .;

    /DISCARD/ : {
            *(.eh_frame)
        }
}