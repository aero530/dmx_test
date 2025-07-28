/*****************************************************************************/
/*  Copyright (c) 2011 NXP B.V.  All rights are reserved.                    */
/*  Reproduction in whole or in part is prohibited without the prior         */
/*  written consent of the copyright owner.                                  */
/*                                                                           */
/*  This software and any compilation or derivative thereof is, and          */
/*  shall remain the proprietary information of NXP and is                   */
/*  highly confidential in nature. Any and all use hereof is restricted      */
/*  and is subject to the terms and conditions set forth in the              */
/*  software license agreement concluded with NXP B.V.                       */
/*                                                                           */
/*  Under no circumstances is this software or any derivative thereof        */
/*  to be combined with any Open Source Software, exposed to, or in any      */
/*  way licensed under any Open License Terms without the express prior      */
/*  written permission of the copyright owner.                               */
/*                                                                           */
/*  For the purpose of the above, the term Open Source Software means        */
/*  any software that is licensed under Open License Terms. Open             */
/*  License Terms means terms in any license that require as a               */
/*  condition of use, modification and/or distribution of a work             */
/*                                                                           */
/*  1. the making available of source code or other materials                */
/*     preferred for modification, or                                        */
/*                                                                           */
/*  2. the granting of permission for creating derivative                    */
/*     works, or                                                             */
/*                                                                           */
/*  3. the reproduction of certain notices or license terms                  */
/*     in derivative works or accompanying documentation, or                 */
/*                                                                           */
/*  4. the granting of a royalty-free license to any party                   */
/*     under Intellectual Property Rights                                    */
/*                                                                           */
/*  regarding the work and/or any work that contains, is combined with,      */
/*  requires or otherwise is based on the work.                              */
/*                                                                           */
/*  This software is provided for ease of recompilation only.                */
/*  Modification and reverse engineering of this software are strictly       */
/*  prohibited.                                                              */
/*                                                                           */
/*****************************************************************************/

#ifndef _BSP_H
#define _BSP_H

#include "app_config.h"
#include "LPC11xx.h"

/* Define the LPC11xx priorities of the supported interrupts */
/* value: 0 .. 3   (0 has highest prio) */

#define I2C_IRQ_PRIORITY     1
#define UART_IRQ_PRIORITY    2
#define SYSTICK_IRQ_PRIORITY 3

#define LPC_DEV_I2C          1
#define LPC_DEV_GPIO         2
#define LPC_DEV_CT16_0       3
#define LPC_DEV_CT16_1       4
#define LPC_DEV_CT32_0       5
#define LPC_DEV_CT32_1       6
#define LPC_DEV_SSP0         7
#define LPC_DEV_UART         8
#define LPC_DEV_ADC          9
#define LPC_DEV_WDT          10
#define LPC_DEV_CAN          11
#define LPC_DEV_SSP1         12

void     bsp_init(void);
void     bsp_control_led(uint8_t led_nr, uint8_t led_state);
void     bsp_recv_from_dmx_bus(void);
void     bsp_xmit_to_dmx_bus(void);
uint8_t  bsp_claim_msec_cnt(void);
void     bsp_reset_msec_cnt(uint8_t counterId);
uint32_t bsp_get_msec_cnt(uint8_t counterId);
void     bsp_get_sys_uptime(uint32_t *psec_cnt, uint16_t *pmsec_cnt);
void     bsp_delay_usec(uint32_t usecDelay);
void     bsp_delay_msec(uint32_t msecDelay);
uint8_t  bsp_read_dip_switches(void);
uint32_t bsp_get_id(void);

#endif
