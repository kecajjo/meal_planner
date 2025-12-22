import sqlite3InitModule from "./sqlite3.js";
import "./sqlite3-opfs-async-proxy.js";

const LOCATE_BASE = "/meal-planner-lib/local-db";
const DEFAULT_DB_NAME = "products.sqlite3";
const DEBUG_LOG = true; // flip to false to disable debug chatter

let sqlite3Promise = null;
let db = null;
let dbName = DEFAULT_DB_NAME;

const locateFile = (file) => `${LOCATE_BASE}/${file}`;

function postResponse(payload) {
    self.postMessage(JSON.stringify(payload));
}

function postError(message) {
    postResponse({ type: "Err", message: String(message) });
}

function postDebug(message) {
    if (!DEBUG_LOG) return;
    postResponse({ type: "Debug", message: String(message) });
}

async function initSqlite() {
    if (!sqlite3Promise) {
        postDebug("init sqlite3 module");
        sqlite3Promise = sqlite3InitModule({
            print: (...args) => console.log("[sqlite3]", ...args),
            printErr: (...args) => console.error("[sqlite3]", ...args),
            locateFile,
        });
    }
    return sqlite3Promise;
}

async function ensureDb(name) {
    if (db) {
        return db;
    }
    postDebug("ensureDb: opening OPFS DB");
    dbName = name || dbName;
    const sqlite3 = await initSqlite();
    if (!sqlite3.oo1 || !sqlite3.oo1.OpfsDb) {
        postDebug("ensureDb: OPFS VFS unavailable");
        throw new Error("OPFS VFS is not available in this environment");
    }
    db = new sqlite3.oo1.OpfsDb(dbName);
    db.exec("PRAGMA foreign_keys=ON;");
    postDebug("ensureDb: DB ready");
    return db;
}

function withTx(fn) {
    db.exec("BEGIN;");
    try {
        const result = fn();
        db.exec("COMMIT;");
        return result;
    } catch (err) {
        db.exec("ROLLBACK;");
        throw err;
    }
}

async function handleMessage(evt) {
    postDebug("handleMessage: received event");
    let req;
    try {
        req = typeof evt.data === "string" ? JSON.parse(evt.data) : JSON.parse(String(evt.data || ""));
    } catch (err) {
        return postError(`Failed to parse request: ${err.message}`);
    }

    try {
        switch (req.type) {
            case "InitDbFile": {
                postDebug("InitDbFile");
                await ensureDb(req.database_file || DEFAULT_DB_NAME);
                return postResponse({ type: "Ok" });
            }
            case "Exec": {
                postDebug("Exec begin");
                await ensureDb(req.database_file || DEFAULT_DB_NAME);
                const statements = req.statements || [];
                withTx(() => {
                    statements.forEach((stmt) => {
                        postDebug(`Exec stmt: ${stmt.sql}`);
                        db.exec({ sql: stmt.sql, bind: stmt.bind || [] });
                    });
                });
                postDebug("Exec done");
                return postResponse({ type: "Ok" });
            }
            case "Query": {
                postDebug("Query begin");
                await ensureDb(req.database_file || DEFAULT_DB_NAME);
                const rows = db.exec({
                    sql: req.sql,
                    bind: req.bind || [],
                    rowMode: "object",
                    returnValue: "resultRows",
                });
                postDebug("Query done");
                return postResponse({ type: "Rows", rows });
            }
            default:
                return postError(`Unknown request type: ${req.type}`);
        }
    } catch (err) {
        postDebug(`Error: ${err?.message || String(err)}`);
        postError(err?.message || String(err));
    }
}

self.addEventListener("message", (evt) => {
    postDebug("worker start listening");
    handleMessage(evt);
});
