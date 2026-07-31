---
id: PROJ-F-0001
kind: 整改
title: payment-service 接入多因素认证
tags: [mfa, payment]
control: "dengbao-2.0:8.1.4.1"
created_at: 2026-07-29
---

负责人:张三;计划:2026 Q3 完成;当前状态:设计中。
方案:复用 user-service 已有的 OTP 能力,接入 payment-service 登录链路。
验收:登录 payment-service 需密码 + OTP;测评时出截图与配置证据。
