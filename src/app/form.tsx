"use client";
import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./form.css";

export default function Form(): React.ReactElement {
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const handleSubmit = (event: React.FormEvent): void => {
    event.preventDefault();
    if (name && email) {
      invoke<string>("greet", { name, email })
        .then((result: string) => console.log(result))
        .catch(console.error);
    }
  };
  return (
    <div className="form-container">
      <h1>AniChain</h1>
      <form onSubmit={handleSubmit}>
        <div className="form-group">
          <label htmlFor="name">Name</label>
          <input
            id="name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
            autoComplete="name"
          />
        </div>
        <div className="form-group">
          <label htmlFor="email">Email</label>
          <input
            id="email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
            autoComplete="email"
          />
        </div>
        <button type="submit">Submit</button>
      </form>
    </div>
  );
}
