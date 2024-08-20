OUTPUT_ARCH( "riscv" )

ENTRY( _start )

MEMORY
{
  ram   (wxa!ri) : ORIGIN = 0x80000000, LENGTH = 128M
}


SECTIONS
{
  . = 0x80000000;
  .text :{
    PROVIDE(_text_start = .);
    *(.text.init) *(.text .text.*)
    PROVIDE(_text_end = .);
  } >ram

   PROVIDE(_global_pointer = .);

  .rodata :ALIGN(4K) {
    PROVIDE(_rodata_start = .);
    *(.rodata .rodata.*)
    PROVIDE(_rodata_end = .);
  } >ram

  .data :ALIGN(4K) {
    PROVIDE(_data_start = .);
    *(.sdata .sdata.*) *(.data .data.*)
    PROVIDE(_data_end = .);
  } >ram

  .bss :ALIGN(4K){
    PROVIDE(_bss_start = .);
    *(.sbss .sbss.*) *(.bss .bss.*)
    PROVIDE(_bss_end = .);
  } >ram

  /DISCARD/ : {
    *(.note.gnu.build-id)
  }

  PROVIDE(_memory_start = ORIGIN(ram));
  PROVIDE(_stack = _bss_end + 0x80000);
  PROVIDE(_memory_end = ORIGIN(ram) + LENGTH(ram));
  PROVIDE(_heap_start = _stack);
  PROVIDE(_heap_size = _memory_end - _stack);
}

