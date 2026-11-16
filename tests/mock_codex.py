#!/usr/bin/python3
"""Offline app-server fixture, launched only by tests via --codex."""
import json
import os
import sys


def send(message):
    print(json.dumps(message, ensure_ascii=False), flush=True)


THREAD = os.environ.get("CODEX_MINI_MOCK_THREAD", "mock-thread")

skill_enabled = True
active_turn = None
user_serial = 0
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
        if method == "thread/resume":
            THREAD = params["threadId"]
        send({"id": request_id, "result": {"thread": {"id": THREAD}, "model": "mock-model"}})
    elif method == "turn/start":
        assert params["threadId"] == THREAD
        prompt = params["input"][0]["text"]
        active_turn = "mock-turn"
        user_serial += 1
        send({"id": request_id, "result": {"turn": {"id": "mock-turn", "status": "inProgress"}}})
        send({"method": "turn/started", "params": {"threadId": THREAD, "turn": {"id": "mock-turn"}}})
        send({"method": "item/completed", "params": {"threadId": THREAD, "item": {"id": f"user-{user_serial}", "clientId": params.get("clientUserMessageId"), "type": "userMessage", "content": [{"text": prompt}]}}})
        if prompt == "hold":
            continue
        if "approval" in prompt:
            send({"id": "approval-1", "method": "item/commandExecution/requestApproval", "params": {"threadId": THREAD, "turnId": "mock-turn", "itemId": "cmd-1", "command": "printf hello", "cwd": "/tmp", "reason": "Offline test only"}})
        else:
            for delta in ["안녕하세요. ", "Codex Mini 테스트 응답입니다."]:
                send({"method": "item/agentMessage/delta", "params": {"threadId": THREAD, "itemId": "assistant-1", "delta": delta}})
            send({"method": "item/completed", "params": {"threadId": THREAD, "item": {"id": "assistant-1", "type": "agentMessage", "text": "안녕하세요. Codex Mini 테스트 응답입니다."}}})
            send({"method":"thread/tokenUsage/updated","params":{"threadId":THREAD,"turnId":"mock-turn","tokenUsage":{"total":{"inputTokens":24000,"cachedInputTokens":16000,"outputTokens":1200,"reasoningOutputTokens":900,"totalTokens":25200},"last":{"inputTokens":24000,"cachedInputTokens":16000,"outputTokens":1200,"reasoningOutputTokens":900,"totalTokens":25200}}}})
            send({"method": "turn/completed", "params": {"threadId": THREAD, "turn": {"id": "mock-turn", "status": "completed"}}})
            active_turn = None
    elif method == "test/finish":
        send({"method": "item/agentMessage/delta", "params": {"threadId": THREAD, "itemId": "background-answer", "delta": "Background reply"}})
        send({"method": "turn/completed", "params": {"threadId": THREAD, "turn": {"id": "mock-turn", "status": "completed"}}})
        active_turn = None
    elif method == "test/question":
        send({"id": "question-1", "method": "item/tool/requestUserInput", "params": {"threadId": THREAD, "turnId": "mock-turn", "questions": [{"id": "q", "question": "Choose a direction", "options": [{"label": "Continue", "description": "Continue work"}]}]}})
    elif method == "turn/steer":
        assert set(params) == {"threadId", "expectedTurnId", "input", "clientUserMessageId"}
        prompt = params["input"][0]["text"]
        if active_turn is None or params["expectedTurnId"] != active_turn or prompt == "reject":
            send({"id": request_id, "error": {"code": -32600, "message": "No matching active turn"}})
            continue
        assert params["threadId"] == THREAD
        user_serial += 1
        send({"id": request_id, "result": {"turnId": active_turn}})
        send({"method": "item/completed", "params": {"threadId": THREAD, "item": {"id": f"user-{user_serial}", "clientId": params["clientUserMessageId"], "type": "userMessage", "content": [{"text": prompt}]}}})
        send({"method": "item/agentMessage/delta", "params": {"threadId": THREAD, "itemId": "assistant-steer", "delta": "추가 메시지 반영: " + prompt}})
    elif method == "thread/compact/start":
        send({"id": request_id, "result": {}})
        send({"method":"turn/started","params":{"threadId":params["threadId"],"turn":{"id":"compact"}}})
        send({"method":"item/completed","params":{"threadId":params["threadId"],"item":{"id":"compaction","type":"contextCompaction"}}})
        send({"method":"turn/completed","params":{"threadId":params["threadId"],"turn":{"id":"compact","status":"completed"}}})
    elif method == "turn/interrupt":
        send({"id": request_id, "result": {}})
        send({"method": "turn/completed", "params": {"threadId": THREAD, "turn": {"id": "mock-turn", "status": "interrupted"}}})
    elif isinstance(request_id, str) and request_id.startswith("auto-test-"):
        send({"method": "test/approval", "params": {"id": request_id, "result": message["result"]}})
    elif request_id == "approval-1":
        assert message["result"]["decision"] in ("accept", "decline")
        send({"method": "serverRequest/resolved", "params": {"threadId": THREAD, "requestId": request_id}})
        send({"method": "turn/completed", "params": {"threadId": THREAD, "turn": {"id": "mock-turn", "status": "completed"}}})
    elif method == "thread/turns/list":
        send({"id": request_id, "result": {"data": [{"items": [{"id": "old-1", "type": "agentMessage", "text": "복원한 대화입니다."}]}], "nextCursor": None}})
    elif request_id is not None:
        send({"id": request_id, "result": {}})
