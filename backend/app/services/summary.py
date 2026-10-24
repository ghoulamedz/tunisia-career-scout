"""
LLM summarization service — generates summaries + required documents for postings.
Supports both API-based (OpenAI/Anthropic) and local (Ollama) models.
"""

from typing import Any

import httpx

from ..core.config import get_settings

settings = get_settings()


async def _call_ollama(prompt: str) -> str:
    """Call local Ollama API."""
    base_url = settings.LLM_BASE_URL
    model = settings.LLM_MODEL

    # If OpenAI-compatible endpoint, use it
    if "/v1/" in base_url:
        headers = {"Authorization": f"Bearer {settings.LLM_API_KEY}"} if settings.LLM_API_KEY else {}
        async with httpx.AsyncClient(timeout=60) as client:
            response = await client.post(
                f"{base_url}/chat/completions",
                headers=headers,
                json={
                    "model": model,
                    "messages": [
                        {"role": "user", "content": prompt}
                    ],
                    "temperature": settings.LLM_TEMPERATURE,
                },
            )
            response.raise_for_status()
            data = response.json()
            return data["choices"][0]["message"]["content"]

    # Standard Ollama format
    headers = {"Content-Type": "application/json"}
    async with httpx.AsyncClient(timeout=60) as client:
        response = await client.post(
            f"{base_url}/api/generate",
            headers=headers,
            json={
                "model": model,
                "prompt": prompt,
                "stream": False,
                "options": {
                    "temperature": settings.LLM_TEMPERATURE,
                },
            },
        )
        response.raise_for_status()
        data = response.json()
        return data.get("response", "")


async def _call_openai(prompt: str) -> str:
    """Call OpenAI-compatible API."""
    import openai

    base_url = settings.LLM_BASE_URL
    if base_url:
        openai.api_base = base_url
    openai.api_key = settings.LLM_API_KEY or "sk-playground-dummy"

    client = openai.AsyncOpenAI()
    response = await client.chat.completions.create(
        model=settings.LLM_MODEL,
        messages=[{"role": "user", "content": prompt}],
        temperature=settings.LLM_TEMPERATURE,
    )
    return response.choices[0].message.content


def _join(lst, default="aucune spécifique"):
    if not lst:
        return default
    return ", ".join(str(x) for x in lst[:3])


async def generate_summary(
    posting_data: dict,
    user_profile: dict,
    posting_type: str = "job",
    include_documents: bool = True,
) -> dict:
    """
    Generate a summary and extract required documents.

    args:
        posting_data: the posting's raw data dict
        user_profile: user resume parsed data
        posting_type: "job" or "concours"
        include_documents: whether to extract required documents

    returns:
        {summary, alignment, relevance, is_relevant, required_documents, raw_output}
    """
    title = posting_data.get("title", "Titre non disponible")
    org = posting_data.get("company") or posting_data.get("organization", "Organisation non identifiée")
    description = posting_data.get("description", "")
    details = posting_data.get("raw", {})

    # Build diploma/level info
    required_diploma = details.get("required_diploma", "") or ""
    required_level = details.get("required_level", "") or ""
    exam_type = details.get("exam_type", "") or ""

    skills = user_profile.get("skills", []) or []
    education = user_profile.get("education", []) or []
    experience = user_profile.get("experience", []) or []

    # Build user profile context text
    skills_str = _join(skills)
    education_str = _join(education)
    experience_str = _join(experience)
    profile_context = f"Vous avez les compétences suivantes : {skills_str}. Votre niveau d'éducation : {education_str}. Votre expérience : {experience_str}."

    # Build the prompt
    prompt = f"""
Tu es un assistant carrière pour les utilisateurs tunisiens.
Ton rôle : analyser une offre de concours ou d'emploi et l'adapter au profil de l'utilisateur.

---
OFFRE :
Titre : {title}
Organisation : {org}
Type : {posting_type}
Description : {description[:3000]}

Diplômes requis : {required_diploma if required_diploma else "Non spécifié"}
Niveau requis : {required_level if required_level else "Non spécifié"}
Type d'examen : {exam_type if exam_type else "Non spécifié"}

---
PROFIL UTILISATEUR :
{profile_context}
---

TÂCHES à effectuer (en français, clair et concis) :

1. RESUMÉ (3-4 phrases maximum) :
   - Qu'est-ce que cette offre propose ?
   - Quelle est l'organisme ou l'entreprise ?
   - Quels sont les objectifs et le contexte ?

2. ALIGNEMENT AVEC LE PROFIL :
   - Qu'est-ce qui correspond au profil de l'utilisateur ?
   - Cite des compétences ou situations spécifiques.

3. CLASSEMENT DE PERTINENCE :
   - Donne un score de pertinence de 0 à 100.

4. DOCUMENTS REQUIS :
   Liste le/des documents requis (diplômes, attestations, etc.) en puces.

Format de sortie :
Résumé: {{{{...}}}}
Alignement: {{{{...}}}}
Pertinence (0-100): {{{{...}}}}
Documents requis: {{{{...}}}}

RÉPONSE:
""".strip()

    # Call the model
    try:
        if settings.LLM_PROVIDER == "ollama":
            raw_output = await _call_ollama(prompt)
        else:
            raw_output = await _call_openai(prompt)

        # Parse the output
        summary = ""
        alignment = ""
        relevance = 50
        documents = []

        for line in raw_output.split("\n"):
            line = line.strip()
            if line.startswith("Résumé:"):
                summary = line.split(":", 1)[1].strip()
            elif line.startswith("Alignement:"):
                alignment = line.split(":", 1)[1].strip()
            elif "Pertinence" in line and "(" in line and ")" in line:
                try:
                    relevance = float(line.split("(")[1].split(")")[0])
                except (ValueError, IndexError):
                    pass
            elif line.startswith("Documents requis:"):
                documents_text = line.split(":", 1)[1].strip()
                if documents_text:
                    documents = [d.strip() for d in documents_text.split("\n") if d.strip()]

        return {
            "summary": summary,
            "alignment": alignment,
            "relevance": min(max(relevance, 0), 100),
            "is_relevant": relevance >= 50,
            "required_documents": documents,
            "raw_output": raw_output[:2000],
        }

    except Exception as e:
        return {
            "summary": "Erreur lors de la génération du résumé.",
            "alignment": f"Impossible de traiter la demande : {str(e)}",
            "relevance": 0,
            "is_relevant": False,
            "required_documents": [],
            "raw_output": f"ERROR: {str(e)}",
        }
