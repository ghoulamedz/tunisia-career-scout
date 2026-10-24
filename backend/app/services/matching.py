"""
Matching engine — TF-IDF cosine similarity between user profile and postings.
"""

import math
from typing import Any

from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.metrics.pairwise import cosine_similarity


def _tfidf_score(user_vec, doc_vec, vocab):
    """Compute a simple cosine similarity using TF-IDF vectors."""
    # Combine text from both sources
    combined = [user_vec, doc_vec]
    vectorizer = TfidfVectorizer()
    tfidf = vectorizer.fit_transform(combined)
    similarity = cosine_similarity(tfidf[0:1], tfidf[1:2])

    # Normalize to 0-100
    return float(similarity[0][0] * 100)


def _extract_skills_from_posting(p: dict, field: str) -> list[str]:
    """Extract skills from a posting JSON field."""
    skills = []
    if field in p and isinstance(p[field], str):
        skills.extend([s.strip() for s in p[field].split(",")])
    elif field in p and isinstance(p[field], list):
        skills.extend([s for s in p[field] if isinstance(s, str)])
    return skills


def _extract_skills_from_resume(resume: dict) -> list[str]:
    """Extract skills from parsed resume data."""
    skills = set()
    for key in ["skills", "competences", "competences_pro"]:

        if key in resume and isinstance(resume[key], list):
            for s in resume[key]:
                if isinstance(s, str):
                    skills.add(s.strip().lower())
        elif key in resume and isinstance(resume[key], str):
            skills.update([s.strip().lower() for s in resume[key].split(",")])

    # Also look at raw text
    if "raw_text" in resume and isinstance(resume["raw_text"], str):
        for line in resume["raw_text"].lower().split("\n"):
            line = line.strip()
            if len(line) > 3:
                # Heuristic: a line that's not a sentence start might be a skill
                pass

    return list(skills)


def _get_user_profile_text(resume: dict) -> str:
    """Build text representation of user profile for TF-IDF."""
    parts = []

    if "skills" in resume and isinstance(resume["skills"], list):
        parts.extend([s for s in resume["skills"] if isinstance(s, str)])

    if "competences" in resume and isinstance(resume["competences"], list):
        parts.extend([s for s in resume["competences"] if isinstance(s, str)])

    if "raw_text" in resume and isinstance(resume["raw_text"], str):
        parts.append(resume["raw_text"])

    if "education" in resume and isinstance(resume["education"], list):
        parts.extend([str(e) for e in resume["education"]])

    if "experience" in resume and isinstance(resume["experience"], list):
        parts.extend([str(e) for e in resume["experience"]])

    return " ".join(parts)


def _get_posting_text(p: dict, posting_type: str) -> str:
    """Build text representation of a posting."""
    parts = []

    if posting_type == "job_posting":
        if "title" in p and isinstance(p["title"], str):
            parts.append(p["title"])
        if "description" in p and isinstance(p["description"], str):
            parts.append(p["description"])
        if "company" in p and isinstance(p["company"], str):
            parts.append(p["company"])
        # Keywords
        for field in ["required_skills", "competences", "missions", "missions_pro"]:
            if field in p and isinstance(p[field], list):
                parts.extend([s for s in p[field] if isinstance(s, str)])
            elif field in p and isinstance(p[field], str):
                parts.append(p[field])

    elif posting_type == "concours_posting":
        if "title" in p and isinstance(p["title"], str):
            parts.append(p["title"])
        if "description" in p and isinstance(p["description"], str):
            parts.append(p["description"])
        if "organization" in p and isinstance(p["organization"], str):
            parts.append(p["organization"])
        if "required_diploma" in p and isinstance(p["required_diploma"], str):
            parts.append(p["required_diploma"])
        if "required_level" in p and isinstance(p["required_level"], str):
            parts.append(p["required_level"])

    return " ".join(parts)


def compute_matches(
    resume_data: dict,
    job_postings: list,
    concours_postings: list,
    user_id: str,
) -> list[dict]:
    """
    Compute matching scores for all postings against the user's resume.
    Returns a list of match dicts with posting metadata.
    """
    if not resume_data:
        return []

    user_text = _get_user_profile_text(resume_data)
    if not user_text.strip():
        return []

    results = []

    for p in job_postings:
        p_text = _get_posting_text(p, "job_posting")
        p_skills = _extract_skills_from_posting(p, "required_skills")

        # Combine text + skills
        combined = user_text + " " + p_text + " " + " ".join(p_skills)

        # Simple scoring: TF-IDF cosine + skill overlap
        # Use a combined approach
        user_terms = set(user_text.lower().split())
        posting_terms = set(p_text.lower().split())
        skill_overlap = set(p_skills) & set(s.strip().lower() for s in user_text.split())

        # TF-IDF cosine similarity (sklearn)
        try:
            vectorizer = TfidfVectorizer()
            tfidf_matrix = vectorizer.fit_transform([user_text, p_text])
            cos_sim = cosine_similarity(tfidf_matrix[0:1], tfidf_matrix[1:2])
            base_score = float(cos_sim[0][0]) * 100
        except Exception:
            base_score = 0.0

        # Skill overlap bonus
        skill_bonus = min(len(skill_overlap) * 15, 40)
        final_score = min(base_score + skill_bonus, 100)

        # Reasons
        reasons = []
        matched_skills = sorted(skill_overlap)

        if matched_skills:
            # Check if any matched skill is in posting's required skills
            if p_skills:
                for sk in matched_skills:
                    if sk in [s.lower() for s in p_skills]:
                        reasons.append(f"Compétence correspondante : {sk}")
                        break

        # Education/experience check
        if "expérience" in user_text.lower():
            reasons.append("Votre expérience professionnelle est pertinente")

        if "diplôme" in user_text.lower():
            reasons.append("Votre diplôme correspond au niveau requis")

        if not matched_skills and not reasons:
            reasons.append("Aucune compétence spécifique identifiée, mais correspondance globale possible")

        # Build match record
        record = {
            "posting_id": str(p["id"]),
            "posting_type": "job_posting",
            "posting_title": p.get("title", ""),
            "score": round(final_score, 1),
            "matched_skills": matched_skills,
            "reasons": reasons,
            "company": p.get("company", ""),
            "source_name": p.get("source_name", ""),
        }
        results.append(record)

    for p in concours_postings:
        p_text = _get_posting_text(p, "concours_posting")
        p_skills = _extract_skills_from_posting(p, "required_skills")

        user_terms = set(user_text.lower().split())
        posting_terms = set(p_text.lower().split())
        skill_overlap = set(p_skills) & set(s.strip().lower() for s in user_text.split())

        try:
            vectorizer = TfidfVectorizer()
            tfidf_matrix = vectorizer.fit_transform([user_text, p_text])
            cos_sim = cosine_similarity(tfidf_matrix[0:1], tfidf_matrix[1:2])
            base_score = float(cos_sim[0][0]) * 100
        except Exception:
            base_score = 0.0

        skill_bonus = min(len(skill_overlap) * 15, 40)
        final_score = min(base_score + skill_bonus, 100)

        reasons = []
        matched_skills = sorted(skill_overlap)

        if matched_skills:
            if p_skills:
                for sk in matched_skills:
                    if sk in [s.lower() for s in p_skills]:
                        reasons.append(f"Compétence correspondante : {sk}")
                        break

        # Education level match
        if "education" in resume_data and isinstance(resume_data["education"], list):
            edu = resume_data["education"][0] if resume_data["education"] else ""
            if edu and p.get("required_level"):
                if edu.lower() in p["required_level"].lower():
                    reasons.append(f"Votre niveau {edu} correspond au niveau requis")

        if not matched_skills and not reasons:
            reasons.append("Aucune compétence spécifique identifiée, mais correspondance globale possible")

        record = {
            "posting_id": str(p["id"]),
            "posting_type": "concours_posting",
            "posting_title": p.get("title", ""),
            "score": round(final_score, 1),
            "matched_skills": matched_skills,
            "reasons": reasons,
            "organization": p.get("organization", ""),
            "source_name": p.get("source_name", ""),
        }
        results.append(record)

    # Sort by score descending and return top results
    results.sort(key=lambda x: x["score"], reverse=True)
    return results
