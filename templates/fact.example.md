---
id: SYS-F-0001
domain: 架构
title: 微服务拓扑
tags: [microservices, api-gateway]
source:
  type: doc
  ref: 架构设计文档 v1.2
  collected_at: 2026-07-29
  collector: agent
confidence: high
related_controls:
  - "dengbao-2.0:8.1.2"
  - "dengbao-2.0:8.1.3"
status: current
---

系统由 user-service / order-service / payment-service 三个微服务组成,
统一经 API 网关接入,服务间走内网 mTLS,对外仅暴露网关 443 端口。
部署于阿里云杭州区域,跨可用区高可用。
