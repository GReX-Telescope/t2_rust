CREATE TABLE t2_cands (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    mjds REAL NOT NULL,
    snr REAL NOT NULL,
    ibox INT NOT NULL,
    dm REAL NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);