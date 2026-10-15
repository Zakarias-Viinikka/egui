================================================================================
PROJECT ARCHITECTURE BLUEPRINT: AUTONOMOUS MULTI-FILE AI WORKFLOW
================================================================================

1. CORE OBJECTIVE
   Build a local background program ("Orchestrator") that automatically manages 
   a multi-step conversation loop between a local application and the Gemini API. 
   The Orchestrator must independently manage conversation state, batch multiple 
   files into single API payloads, and handle free-tier rate limits without human 
   intervention.

2. LOGICAL COMPONENTS
   A. Trigger Mechanism:
      - App saves files and a master plan to a designated folder or pushes 
        them directly to the Orchestrator.
   B. Context & History Manager:
      - Maintains a local chat history array (`history = []`) because the API 
        is stateless. Each message and response is saved chronologically.
   C. Token/File Optimizer:
      - Packs multiple files into a single string payload using distinct delimiters
        so the AI treats them as separate entities within the 2-million token window.
   D. Rate Limiter (Traffic Controller):
      - Monitors API call frequencies to stay under 15 Requests Per Minute (RPM).
      - Catches 429 errors and applies an automatic wait-and-retry strategy.

3. ARCHITECTURAL FLOW (THE AUTOMATION LOOP)
   [Start Workflow]
          │
          ▼
   ┌────────────────────────────────────────────────────────┐
   │ 1. Read files and master plan prompt from local system │
   └────────────────────────┬───────────────────────────────┘
                            │
                            ▼
   ┌────────────────────────────────────────────────────────┐
   │ 2. Package text structure:                             │
   │    [FILE_START: name] contents [FILE_END: name]        │
   └────────────────────────┬───────────────────────────────┘
                            │
                            ▼
   ┌────────────────────────────────────────────────────────┐
   │ 3. Append previous conversation history array to data │
   └────────────────────────┬───────────────────────────────┘
                            │
                            ▼
   ┌────────────────────────────────────────────────────────┐
   │ 4. Check Rate Limiter: Has it been < 4 seconds since   │
   │    the last call? If yes, pause/sleep.                 │
   └────────────────────────┬───────────────────────────────┘
                            │
                            ▼
   ┌────────────────────────────────────────────────────────┐
   │ 5. Execute API Call (Gemini 1.5 Flash - Free Tier)    │
   └────────────────────────┬───────────────────────────────┘
                            │
              ┌─────────────┴─────────────┐
              ▼                           ▼
      [Success (HTTP 200)]        [Rate Limited (HTTP 429)]
              │                           │
              ▼                           ▼
   ┌─────────────────────┐     ┌────────────────────────────┐
   │ Extract AI reply    │     │ Trigger Exponential        │
   │ Update local history│     │ Backoff: Sleep 5-10 secs,  │
   └──────────┬──────────┘     │ loop back to Step 5.       │
              │                └────────────────────────────┘
              ▼
   ┌────────────────────────────────────────────────────────┐
   │ 6. Process AI response: Is the task complete?          │
   └────────────────────────┬───────────────────────────────┘
              ┌─────────────┴─────────────┐
              ▼                           ▼
           [No]                         [Yes]
              │                           │
              ▼                           ▼
   ┌─────────────────────┐     ┌────────────────────────────┐
   │ Formulate next query│     │ Output final result to local│
   │ Loop back to Step 4.│     │ file. End background worker│
   └─────────────────────┘     └────────────────────────────┘

4. TECHNICAL SPECIFICATIONS & EDGE CASES
   - System Instructions: hardcode a global role in Google AI Studio to keep the 
     agent aligned across the loop (e.g., "Analyze the files given to you according 
     to the user's plan. Focus strictly on executing tasks sequentially.")
   - Rate Limits: Target maximum 15 Requests Per Minute (RPM) and 1,000,000 
     Tokens Per Minute (TPM).
   - Error Handling: Use standard try/catch blocks wrapped around the network 
     request to prevent the entire program from crashing on bad network requests.
================================================================================
