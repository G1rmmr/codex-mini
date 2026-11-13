#!/usr/bin/python3
"""Offline app-server fixture, launched only by tests via --codex."""
import json
import os
import sys


def send(message):
    print(json.dumps(message, ensure_ascii=False), flush=True)


skill_enabled = True
for line in sys.stdin:
    message = json.loads(line)
    method = message.get("method")
    request_id = message.get("id")
    params = message.get("params", {})
    if method == "initialize":
        assert params["clientInfo"]["name"] == "codex_mini"
        send({"id": request_id, "result": {"userAgent": "mock"}})
    elif method == "account/read":
        send({"id": request_id, "result": {"account": {"type": "chatgpt", "planType": "test"}}})
    elif method == "config/read":
        send({"id":request_id,"result":{"config":{},"origins":{}}})
    elif method == "skills/list":
        send({"id":request_id,"result":{"data":[{"cwd":params.get("cwds",["/tmp"])[0],"errors":[],"skills":[{"name":"egui-ui-design","path":os.path.join(os.environ.get("CODEX_HOME","/tmp"),"skills/egui-ui-design/SKILL.md"),"description":"egui 화면의 크기, 정렬, 대비를 검증합니다.","scope":"user","enabled":skill_enabled}]}]}})
    elif method == "skills/config/write":
        skill_enabled=params["enabled"]
        send({"id":request_id,"result":{}})
    elif method == "model/list":
        send({"id": request_id, "result": {"data": [{"model": "mock-model", "supportedReasoningEfforts": [{"reasoningEffort": "low"}]}]}})
    elif method in ("thread/start", "thread/resume"):
        assert params.get("sandbox", "workspace-write") == "workspace-write"
        assert params.get("approvalPolicy", "on-request") == "on-request"
        send({"id": request_id, "result": {"thread": {"id": "mock-thread"}, "model": "mock-model"}})
    elif method == "turn/start":
        assert params["threadId"] == "mock-thread"
        prompt = params["input"][0]["text"]
        send({"id": request_id, "result": {"turn": {"id": "mock-turn", "status": "inProgress"}}})
        send({"method": "turn/started", "params": {"threadId": "mock-thread", "turn": {"id": "mock-turn"}}})
        send({"method": "item/completed", "params": {"threadId": "mock-thread", "item": {"id": "user-1", "type": "userMessage", "content": [{"text": prompt}]}}})
        if "approval" in prompt:
            send({"id": "approval-1", "method": "item/commandExecution/requestApproval", "params": {"threadId": "mock-thread", "turnId": "mock-turn", "itemId": "cmd-1", "command": "printf hello", "cwd": "/tmp", "reason": "Offline test only"}})
        else:
            for delta in ["안녕하세요. ", "Codex Mini 테스트 응답입니다."]:
                send({"method": "item/agentMessage/delta", "params": {"threadId": "mock-thread", "itemId": "assistant-1", "delta": delta}})
            send({"method": "item/completed", "params": {"threadId": "mock-thread", "item": {"id": "assistant-1", "type": "agentMessage", "text": "안녕하세요. Codex Mini 테스트 응답입니다."}}})
            send({"method":"thread/tokenUsage/updated","params":{"threadId":"mock-thread","turnId":"mock-turn","tokenUsage":{"total":{"inputTokens":24000,"cachedInputTokens":16000,"outputTokens":1200,"reasoningOutputTokens":900,"totalTokens":25200},"last":{"inputTokens":24000,"cachedInputTokens":16000,"outputTokens":1200,"reasoningOutputTokens":900,"totalTokens":25200}}}})
            send({"method": "turn/completed", "params": {"threadId": "mock-thread", "turn": {"id": "mock-turn", "status": "completed"}}})
    elif method == "turn/interrupt":
        send({"id": request_id, "result": {}})
        send({"method": "turn/completed", "params": {"threadId": "mock-thread", "turn": {"id": "mock-turn", "status": "interrupted"}}})
    elif request_id == "approval-1":
        assert message["result"]["decision"] in ("accept", "decline")
        send({"method": "serverRequest/resolved", "params": {"threadId": "mock-thread", "requestId": request_id}})
        send({"method": "turn/completed", "params": {"threadId": "mock-thread", "turn": {"id": "mock-turn", "status": "completed"}}})
    elif method == "thread/turns/list":
        send({"id": request_id, "result": {"data": [{"items": [{"id": "old-1", "type": "agentMessage", "text": "복원한 대화입니다."}]}], "nextCursor": None}})
    elif request_id is not None:
        send({"id": request_id, "result": {}})
