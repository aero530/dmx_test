/*******************************************************************************
*   wdt.c:  WDT API file for NXP LPC11xx
*
*   Copyright(C) 2011, NXP Semiconductor
*   All rights reserved.
*
*   History
*
*******************************************************************************/

#include "bsp.h"
#include "wdt.h"

#define WDEN     0x00000001
#define WDRESET  0x00000002
#define WDTOF    0x00000004
#define WDINT    0x00000008


/*******************************************************************************
* Function name: WDT_Init
* Description  : Initialize watchdog timer
*                 The watchdog timeout = Twdtclk * WDT_FEED_VALUE * 4
*                 The AHB clock is used for Twdtclk
* Argument(s)  : timeout - time after which watchdog kicks in
* Return(s)    : none.
*******************************************************************************/

void WDT_Init(uint32_t timeout)
{
#ifndef DEBUG
	LPC_WDT->TC  = (WDT_clk_get() / 4) * timeout;
    /* once WDEN is set, the WDT will start after feeding */
	LPC_WDT->MOD = WDEN | WDRESET;
#endif
}

/*******************************************************************************
* Function name: WDT_Feed
* Description  : Feed watchdog timer to prevent it from timeout
* Argument(s)  : none.
* Return(s)    : none.
*******************************************************************************/

void WDT_Feed(void)
{
#ifndef DEBUG
    LPC_WDT->FEED = 0xAA;   /* Feeding sequence */
    LPC_WDT->FEED = 0x55;
#endif
}

/* End Of File */
