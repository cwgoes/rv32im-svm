/* Linker script for Real Compiler benchmark */
ENTRY(_start)

SECTIONS
{
    . = 0x0000;

    .text : {
        /* Put startup code first */
        *(.text._start)
        *start.o(.text)

        /* Then the rest of the code */
        *(.text)
        *(.text.*)
    }

    .rodata : {
        *(.rodata)
        *(.rodata.*)
    }

    .data : {
        *(.data)
        *(.data.*)
    }

    .bss : {
        *(.bss)
        *(.bss.*)
    }
}
