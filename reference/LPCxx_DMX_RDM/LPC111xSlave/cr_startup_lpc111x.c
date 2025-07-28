//*****************************************************************************
//   +--+       
//   | ++----+   
//   +-++    |  
//     |     |  
//   +-+--+  |   
//   | +--+--+  
//   +----+    Copyright (c) 2009-10 Code Red Technologies Ltd.
//
// Microcontroller Startup code for use with Red Suite
//
// Software License Agreement
// 
// The software is owned by Code Red Technologies and/or its suppliers, and is 
// protected under applicable copyright laws.  All rights are reserved.  Any 
// use in violation of the foregoing restrictions may subject the user to criminal 
// sanctions under applicable laws, as well as to civil liability for the breach 
// of the terms and conditions of this license.
// 
// THIS SOFTWARE IS PROVIDED "AS IS".  NO WARRANTIES, WHETHER EXPRESS, IMPLIED
// OR STATUTORY, INCLUDING, BUT NOT LIMITED TO, IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE APPLY TO THIS SOFTWARE.
// USE OF THIS SOFTWARE FOR COMMERCIAL DEVELOPMENT AND/OR EDUCATION IS SUBJECT
// TO A CURRENT END USER LICENSE AGREEMENT (COMMERCIAL OR EDUCATIONAL) WITH
// CODE RED TECHNOLOGIES LTD. 
//
//*****************************************************************************
#if defined (__cplusplus)
#ifdef __REDLIB__
#error Redlib does not support C++
#else
//*****************************************************************************
//
// The entry point for the C++ library startup
//
//*****************************************************************************
extern "C" {
	extern void __libc_init_array(void);
}
#endif
#endif

#define WEAK __attribute__ ((weak))
#define ALIAS(f) __attribute__ ((weak, alias (#f)))

// Code Red - if CMSIS is being used, then SystemInit() routine
// will be called by startup code rather than in application's main()
#if defined (__USE_CMSIS)
#include "system_LPC11xx.h"
#endif

//*****************************************************************************
#if defined (__cplusplus)
extern "C" {
#endif

//*****************************************************************************
//
// Forward declaration of the default handlers. These are aliased.
// When the application defines a handler (with the same name), this will 
// automatically take precedence over these weak definitions
//
//*****************************************************************************
     void ResetISR(void);
WEAK void NMI_Handler(void);
WEAK void HardFault_Handler(void);
WEAK void SVC_Handler(void);
WEAK void PendSV_Handler(void);
WEAK void TickHandler(void);
WEAK void IntDefaultHandler(void);
//*****************************************************************************
//
// Forward declaration of the specific IRQ handlers. These are aliased
// to the IntDefaultHandler, which is a 'forever' loop. When the application
// defines a handler (with the same name), this will automatically take
// precedence over these weak definitions
//
//*****************************************************************************

void CAN_IRQHandler (void) ALIAS(IntDefaultHandler);
void SSP1_IRQHandler (void) ALIAS(IntDefaultHandler);
void I2C_IRQHandler (void) ALIAS(IntDefaultHandler);
void TIMER16_0_IRQHandler (void) ALIAS(IntDefaultHandler);
void TIMER16_1_IRQHandler (void) ALIAS(IntDefaultHandler);
void TIMER32_0_IRQHandler (void) ALIAS(IntDefaultHandler);
void TIMER32_1_IRQHandler (void) ALIAS(IntDefaultHandler);
void SSP0_IRQHandler (void) ALIAS(IntDefaultHandler);
void UART_IRQHandler (void) ALIAS(IntDefaultHandler);
void ADC_IRQHandler (void) ALIAS(IntDefaultHandler);
void WDT_IRQHandler (void) ALIAS(IntDefaultHandler);
void BOD_IRQHandler (void) ALIAS(IntDefaultHandler);
void PIOINT3_IRQHandler (void) ALIAS(IntDefaultHandler);
void PIOINT2_IRQHandler (void) ALIAS(IntDefaultHandler);
void PIOINT1_IRQHandler (void) ALIAS(IntDefaultHandler);
void GPIO_IRQHandler (void) ALIAS(IntDefaultHandler);
void PIOINT0_IRQHandler (void) ALIAS(IntDefaultHandler);
void WAKEUP_IRQHandler (void) ALIAS(IntDefaultHandler);

//*****************************************************************************
//
// The entry point for the application.
// __main() is the entry point for redlib based applications
// main() is the entry point for newlib based applications
//
//*****************************************************************************
//
// The entry point for the application.
// __main() is the entry point for Redlib based applications
// main() is the entry point for Newlib based applications
//
//*****************************************************************************
#if defined (__REDLIB__)
extern void __main(void);
#endif
extern int main(void);
//*****************************************************************************
//
// External declaration for the pointer to the stack top from the Linker Script
//
//*****************************************************************************
extern void _vStackTop(void);

//*****************************************************************************
#if defined (__cplusplus)
} // extern "C"
#endif
//*****************************************************************************
//
// The vector table.  Note that the proper constructs must be placed on this to
// ensure that it ends up at physical address 0x0000.0000.
//
//*****************************************************************************
extern void (* const g_pfnVectors[])(void);
__attribute__ ((section(".isr_vector")))
void (* const g_pfnVectors[])(void) = {
    &_vStackTop,		    				// The initial stack pointer
    ResetISR,                               // Exception #1:  The reset handler
    NMI_Handler,                            // Exception #2:  The NMI handler
    HardFault_Handler,                      // Exception #3:  The hard fault handler
    0,                      				// Exception #4:  Reserved
    0,                      				// Exception #5:  Reserved
    0,                      				// Exception #6:  Reserved
    0,                                      // Exception #7:  Reserved
    0,                                      // Exception #8:  Reserved
    0,                                      // Exception #9:  Reserved
    0,                                      // Exception #10: Reserved
    SVC_Handler,                         	// Exception #11: SVCall handler
    0,                      				// Exception #12: Reserved
    0,                                      // Exception #13: Reserved
    PendSV_Handler,                         // Exception #14: The PendSV handler
    TickHandler,                            // Exception #15: The SysTick handler
    WAKEUP_IRQHandler,                      // Exception #16: PIO0_0  Wakeup
    WAKEUP_IRQHandler,                      // Exception #17: PIO0_1  Wakeup
    WAKEUP_IRQHandler,                      // Exception #18: PIO0_2  Wakeup
    WAKEUP_IRQHandler,                      // Exception #19: PIO0_3  Wakeup
    WAKEUP_IRQHandler,                      // Exception #20: PIO0_4  Wakeup
    WAKEUP_IRQHandler,                      // Exception #21: PIO0_5  Wakeup
    WAKEUP_IRQHandler,                      // Exception #22: PIO0_6  Wakeup
    WAKEUP_IRQHandler,                      // Exception #23: PIO0_7  Wakeup
    WAKEUP_IRQHandler,                      // Exception #24: PIO0_8  Wakeup
    WAKEUP_IRQHandler,                      // Exception #25: PIO0_9  Wakeup
    WAKEUP_IRQHandler,                      // Exception #26: PIO0_10 Wakeup
    WAKEUP_IRQHandler,                      // Exception #27: PIO0_11 Wakeup
    WAKEUP_IRQHandler,                      // Exception #28: PIO1_0  Wakeup
    CAN_IRQHandler,							// C_CAN Interrupt
    SSP1_IRQHandler, 						// Exception #30: SSP1_IRQn
    I2C_IRQHandler,                      	// Exception #31: I2C_IRQn
    TIMER16_0_IRQHandler,                   // Exception #32: TIMER_16_0_IRQn
    TIMER16_1_IRQHandler,                   // Exception #33: TIMER_16_1_IRQn
    TIMER32_0_IRQHandler,                   // Exception #34: TIMER_32_0_IRQn
    TIMER32_1_IRQHandler,                   // Exception #35: TIMER_32_1_IRQn
    SSP0_IRQHandler,                      	// Exception #36: SSP0_IRQn
    UART_IRQHandler,                      	// Exception #37: UART_IRQn
    0, 				                     	// Exception #38: Reserved
    0,                      				// Exception #39: Reserved
    ADC_IRQHandler,                      	// Exception #40: ADC_IRQn
    WDT_IRQHandler,                      	// Exception #41: WDT_IRQn
    BOD_IRQHandler,                      	// Exception #42: BOD_IRQn
    0,                      				// Exception #43: Reserved
    PIOINT3_IRQHandler,                     // Exception #44: PIO INT3
    PIOINT2_IRQHandler,                     // Exception #45: PIO INT2
    GPIO_IRQHandler,                        // Exception #46: PIO INT1
    PIOINT0_IRQHandler,                     // Exception #47: PIO INT0
};

//*****************************************************************************
//
// The following are constructs created by the linker, indicating where the
// the "data" and "bss" segments reside in memory.  The initializers for the
// for the "data" segment resides immediately following the "text" segment.
//
//*****************************************************************************
extern unsigned long _etext;
extern unsigned long _data;
extern unsigned long _edata;
extern unsigned long _bss;
extern unsigned long _ebss;

//*****************************************************************************
//
// This is the code that gets called when the processor first starts execution
// following a reset event.  Only the absolutely necessary set is performed,
// after which the application supplied entry() routine is called.  Any fancy
// actions (such as making decisions based on the reset cause register, and
// resetting the bits in that register) are left solely in the hands of the
// application.
//
//*****************************************************************************
void ResetISR(void)
{
    unsigned char *pulSrc, *pulDest;

    //
    // Copy the data segment initializers from flash to SRAM.
    //
    pulSrc = (unsigned char *)&_etext;
    for(pulDest = (unsigned char *)&_data; pulDest < (unsigned char *)&_edata; )
    {
        *pulDest++ = *pulSrc++;
    }

    //
    // Zero fill the bss segment.
    //
	for(pulDest = (unsigned char *)&_bss; pulDest < (unsigned char *)&_ebss; pulDest++)
	  *pulDest = 0;

#ifdef __USE_CMSIS
	SystemInit();
#endif

#if defined (__cplusplus)
	//
	// Call C++ library initialisation
	//
	__libc_init_array();
#endif

#if defined (__REDLIB__)
	// Call the Redlib library, which in turn calls main()
		__main() ;
#else
	main();
#endif
	//
	// main() shouldn't return, but if it does, we'll just enter an infinite loop 
	//
	while (1) {
		;
	}
}

//*****************************************************************************
//
// This is the code that gets called when the processor receives a NMI.  This
// simply enters an infinite loop, preserving the system state for examination
// by a debugger.
//
//*****************************************************************************
void NMI_Handler(void)
{
    //
    // Enter an infinite loop.
    //
    while(1)
    {
    }
}

//*****************************************************************************
//
// This is the code that gets called when the processor receives a fault
// interrupt.  This simply enters an infinite loop, preserving the system state
// for examination by a debugger.
//
//*****************************************************************************
void HardFault_Handler(void)
{
    //
    // Enter an infinite loop.
    //
    while(1)
    {
    }
}

//*****************************************************************************
//
// This is the code that gets called when the processor receives an unexpected
// interrupt.  This simply enters an infinite loop, preserving the system state
// for examination by a debugger.
//
//*****************************************************************************
void IntDefaultHandler(void)
{
    //
    // Go into an infinite loop.
    //
    while(1)
    {
    }
}
