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

#ifndef  __TIMER_PWM_H
#define  __TIMER_PWM_H

#define PWM_FREQ     733 //2200    /* default timer frequency */
#define PWM_DUTY     50            /* default duty cycle 0 - 255 */

// counter timer Id's (=ctID)
#define CT16B0   0
#define CT16B1   1
#define CT32B0   2
#define CT32B1   3

// match output Id's (=matId)
#define CTMAT0   0
#define CTMAT1   1
#define CTMAT2   2
#define CTMAT3   3

void     Timer_PWM_Init(uint8_t ctId);
uint32_t Timer_PWM_Set_Frequency(uint8_t ctId, uint16_t new_freq);
void     Timer_PWM_Set_Duty_Cycle(uint8_t ctId, uint8_t matId, uint8_t duty);

#endif
