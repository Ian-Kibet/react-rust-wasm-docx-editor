import React, { useState } from 'react';
import { Comment, DocxDocument } from '../../../types/document';

interface CommentsPanelProps {
  document: DocxDocument;
  dispatch: React.Dispatch<any>;
}

export const CommentsPanel: React.FC<CommentsPanelProps> = ({ document: doc, dispatch }) => {
  const [newAuthor, setNewAuthor] = useState('User');
  const [newContent, setNewContent] = useState('');
  const comments = doc.comments ?? [];

  const handleAdd = () => {
    if (!newContent.trim()) return;
    dispatch({
      type: 'add_comment',
      payload: { author: newAuthor, content: newContent.trim() },
    });
    setNewContent('');
  };

  const handleDelete = (commentId: string) => {
    dispatch({ type: 'delete_comment', payload: { comment_id: commentId } });
  };

  const formatDate = (date?: string) => {
    if (!date) return '';
    try {
      return new Date(date).toLocaleDateString(undefined, {
        month: 'short',
        day: 'numeric',
        year: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
      });
    } catch {
      return date;
    }
  };

  const getCommentText = (comment: Comment): string => {
    return comment.content
      .filter((b): b is { paragraph: any } => 'paragraph' in b)
      .map((b) => b.paragraph.runs.map((r: any) => r.text).join(''))
      .join('\n');
  };

  return (
    <div className="comments-panel">
      <div className="comments-add">
        <div className="comments-add-row">
          <input
            type="text"
            className="comments-author-input"
            placeholder="Author"
            value={newAuthor}
            onChange={(e) => setNewAuthor(e.target.value)}
          />
        </div>
        <div className="comments-add-row">
          <textarea
            className="comments-content-input"
            placeholder="Add a comment..."
            value={newContent}
            onChange={(e) => setNewContent(e.target.value)}
            rows={2}
          />
        </div>
        <button className="comments-add-btn" onClick={handleAdd} disabled={!newContent.trim()}>
          Add Comment
        </button>
      </div>

      {comments.length === 0 ? (
        <div className="comments-empty">No comments yet</div>
      ) : (
        <div className="comments-list">
          {comments.map((comment) => (
            <div key={comment.id} className="comment-card">
              <div className="comment-header">
                <span className="comment-author">
                  {comment.initials && (
                    <span className="comment-initials">{comment.initials}</span>
                  )}
                  {comment.author}
                </span>
                {comment.date && (
                  <span className="comment-date">{formatDate(comment.date)}</span>
                )}
                <button
                  className="comment-delete"
                  onClick={() => handleDelete(comment.id)}
                  title="Delete comment"
                >
                  \u2715
                </button>
              </div>
              <div className="comment-body">{getCommentText(comment)}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
