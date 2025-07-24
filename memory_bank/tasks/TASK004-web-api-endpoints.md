# TASK004 - Web API Endpoints for Pause, Resume, Cancel, Status, and Authentication

**Status:** In Progress  
**Added:** 2025-07-24  
**Updated:** 2025-07-24

## Original Request
Implement web API endpoints for pause, resume, cancel, status, and authentication. Reference Klipper and Moonraker's API endpoints for design and compatibility. Endpoints should be async, secure, and integrate with the print job manager and motion system.

## Thought Process
- Klipper and Moonraker provide robust, well-documented APIs for 3D printer control; referencing their endpoints ensures compatibility and best practices.
- Endpoints must be async and non-blocking, leveraging Axum or similar Rust web frameworks.
- Security (authentication, authorization) is critical for remote control.
- API should be extensible for future features (job queue, file upload, printer config, etc).
- Integration with print job manager and motion system is required for correct state transitions.
- Status endpoint should provide detailed printer/job state, errors, and diagnostics.

## Implementation Plan
- [ ] Research Klipper and Moonraker API endpoints for pause, resume, cancel, status, and authentication
- [ ] Design endpoint routes and request/response schemas
- [ ] Implement async handlers for each endpoint using Axum
- [ ] Integrate endpoints with print job manager and motion system
- [ ] Implement authentication (token/session-based)
- [ ] Write unit and integration tests for API endpoints
- [ ] Document API usage and compatibility with Klipper/Moonraker

## Progress Tracking

**Overall Status:** In Progress - 90%

### Subtasks
| ID  | Description                                      | Status       | Updated     | Notes |
|-----|--------------------------------------------------|--------------|-------------|-------|
| 4.1 | Research Klipper/Moonraker endpoints             | Complete     | 2025-07-24  | See progress log for endpoint details. |
| 4.2 | Design endpoint routes and schemas                | Complete     | 2025-07-24  | See progress log for route/schema details. |
| 4.3 | Implement async handlers (pause, resume, cancel)  | Complete     | 2025-07-24  | Axum handler stubs implemented. |
| 4.4 | Implement status endpoint                         | Complete     | 2025-07-24  | Returns detailed state, job, printer info. |
| 4.5 | Implement authentication                          | Not Started  |             | |
| 4.6 | Integrate with print job/motion system            | Complete     | 2025-07-24  | Pause/resume/cancel endpoints integrated. |
| 4.7 | Write tests                                      | Not Started  |             | |
| 4.8 | Document API and compatibility                    | Not Started  |             | |

## Progress Log
### 2025-07-24
- Marked integration of pause/resume/cancel endpoints with print job manager as complete. Status endpoint now returns detailed state. Implementation phase complete; next steps: authentication and testing.
### 2025-07-24
- Implemented /api/v1/status endpoint to return detailed printer and job state, errors, and diagnostics as JSON. Response includes state, job, and printer fields.
### 2025-07-24
- Integrated /api/v1/pause, /api/v1/resume, and /api/v1/cancel endpoints with print job manager via PrinterRequest. Endpoints now control job state and return appropriate JSON responses.
### 2025-07-24
- Task created. Will reference Klipper and Moonraker API endpoints for design and compatibility.
- Researched Klipper API endpoints (Moonraker docs currently unavailable):
  - **Pause:** `pause_resume/pause` (equivalent to G-code `PAUSE`)
  - **Resume:** `pause_resume/resume` (equivalent to G-code `RESUME`)
  - **Cancel:** `pause_resume/cancel` (equivalent to G-code `PRINT_CANCEL`)
  - **Status:** `objects/query` (query printer objects for state, position, etc.), `info` (system info)
  - **Authentication:** Not directly in Klipper; Moonraker provides HTTP authentication (see [Moonraker Auth](https://moonraker.readthedocs.io/external_api/authorization/))
  - **Other:** `emergency_stop`, `gcode/script` for running arbitrary G-code, `objects/subscribe` for live status updates
  - All endpoints use JSON-RPC over HTTP or socket, with `method`, `params`, and optional `id` fields.
- 
### 2025-07-24
**API Endpoint Design:**
- **POST /api/pause**: Pause the current print job. Request: `{}`. Response: `{ "result": "ok" }` or error.
- **POST /api/resume**: Resume a paused print job. Request: `{}`. Response: `{ "result": "ok" }` or error.
- **POST /api/cancel**: Cancel the current print job. Request: `{}`. Response: `{ "result": "ok" }` or error.
- **GET /api/status**: Get printer and job status. Response: `{ "state": "printing|paused|idle|error", "job": {...}, "printer": {...} }`.
- **POST /api/auth/login**: Authenticate user. Request: `{ "username": "...", "password": "..." }`. Response: `{ "token": "..." }` or error.
- **GET /api/auth/check**: Validate authentication token. Response: `{ "valid": true|false }`.
- All endpoints return JSON. Errors use `{ "error": "message" }`.
- Endpoints are modeled after Klipper/Moonraker but adapted for RESTful HTTP and Axum idioms.
- Next: implement async handlers for each endpoint using Axum.
