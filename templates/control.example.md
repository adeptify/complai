---
id: "dengbao-2.0:8.1.4.1"
framework: dengbao-2.0
domain: 技术
category: 安全计算环境
control_id: "8.1.4.1"
title: 身份鉴别
levels: [3]
tags: [identity, authentication, access-control]
mappings:
  iso27001: ["A.5.16", "A.8.5"]
expected_evidence:
  - 多因素认证启用截图
  - 密码复杂度策略配置
  - 登录失败锁定配置
  - 鉴别信息传输加密证据
excerpt_status: complete
last_reviewed: 2026-07-29
---

# 身份鉴别

## 要求摘要
对登录系统的用户进行身份鉴别,采用口令或生物等手段;口令需满足复杂度并定期更换;
启用登录失败处理(锁定)与鉴别信息传输加密。

## 实施指引
- 区分用户身份与系统/设备身份
- 高权限账户启用多因素认证
- 鉴别信息传输加密(如 TLS)
- 登录失败达到阈值锁定账户,并记录告警

## 常见缺陷
- 仅弱口令、无多因素
- 鉴别信息明文传输
- 未启用登录失败锁定
- 默认账户未改默认口令
