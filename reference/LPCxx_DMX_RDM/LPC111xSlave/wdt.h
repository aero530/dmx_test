/*****************************************************************************
 *   wdt.h:  Header file for NXP LPC11xx
 *
 *   Copyright(C) 2011, NXP Semiconductor
 *   All rights reserved.
 *
 *   History
 *
 ******************************************************************************/

#ifndef __WDT_H 
#define __WDT_H

void WDT_Init(uint32_t timeout);
void WDT_Feed(void);

#endif /* end __WDT_H */
