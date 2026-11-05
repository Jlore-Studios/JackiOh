// News posts: post one, list every one (newest first, as the server sends them), take one down. The
// server checks a post (its title, its one or two sentences, its lengths) and says what it refused.

import { useId, useState, type ReactElement } from "react";

import { useRead, when, type SectionProps } from "./section.ts";
import { testids } from "./testids.ts";

export const NO_REWARD_NOTE = "A post pays no reward; a reward goes in crates/server/news.json by pull request.";

export function News({ api, revision, working, run }: SectionProps): ReactElement {
  const ids = useId();
  const read = useRead(() => api.news(), [api, revision]);
  const [title, setTitle] = useState("");
  const [summary, setSummary] = useState("");

  return (
    <section data-testid={testids.news} aria-labelledby={`${ids}-heading`}>
      <h2 id={`${ids}-heading`}>News posts</h2>
      <form
        className="stack"
        onSubmit={(event) => {
          event.preventDefault();
          run(async () => {
            await api.postNews(title, summary);
            setTitle("");
            setSummary("");
            return "Posted.";
          });
        }}
      >
        <label htmlFor={`${ids}-title`}>Title</label>
        <input
          id={`${ids}-title`}
          data-testid={testids.newsTitle}
          value={title}
          disabled={working}
          onChange={(event) => {
            setTitle(event.target.value);
          }}
        />
        <label htmlFor={`${ids}-summary`}>Summary (one or two sentences)</label>
        <textarea
          id={`${ids}-summary`}
          data-testid={testids.newsSummary}
          value={summary}
          disabled={working}
          onChange={(event) => {
            setSummary(event.target.value);
          }}
        />
        <p className="note">{NO_REWARD_NOTE}</p>
        <div className="row">
          <button type="submit" data-testid={testids.newsPost} disabled={working}>
            Post
          </button>
        </div>
      </form>

      {read.error === null ? null : (
        <p className="error" data-testid={testids.loadError}>
          {read.error}
        </p>
      )}
      {read.value === null ? (
        <p>…</p>
      ) : read.value.posts.length === 0 ? (
        <p>No posts yet.</p>
      ) : (
        <ul className="posts">
          {read.value.posts.map((post) => (
            <li
              key={post.id}
              data-testid={testids.newsRow}
              data-post-id={post.id}
              data-taken-down={String(post.takenDownAt !== null)}
            >
              <h3>{post.title}</h3>
              <p>{post.summary}</p>
              <p className="note">
                {`Posted ${when(post.postedAt)} by ${post.postedBy === "" ? "an account that is gone" : post.postedBy}`}
                {post.takenDownAt === null ? "" : `. Taken down ${when(post.takenDownAt)}`}
              </p>
              {post.takenDownAt === null ? (
                <button
                  type="button"
                  className="danger"
                  data-testid={testids.newsTakeDown}
                  disabled={working}
                  onClick={() => {
                    run(async () => {
                      await api.takeDownNews(post.id);
                      return "Taken down.";
                    });
                  }}
                >
                  Take down
                </button>
              ) : null}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
