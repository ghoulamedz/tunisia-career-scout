// Classic CV Template - Typst
// Traditional professional layout with clear sections

#let page-margin = 1in

// Theme variables - overridden by data injection
#let primary-color = rgb("#1e3a5f")
#let secondary-color = rgb("#2c5282")
#let text-color = rgb("#1a202c")
#let muted-color = rgb("#718096")
#let line-color = rgb("#e2e8f0")
#let bg-color = rgb("#ffffff")
#let card-bg = rgb("#f7fafc")
#let card-border = rgb("#e2e8f0")

#set page(
  margin: page-margin,
  paper: "a4",
  fill: bg-color,
)

#set text(
  font: "Times New Roman",
  size: 10.5pt,
  fill: text-color,
  line-height: 1.6,
)

// Header with name and contacts
#let header(name, contacts) = {
  align(center, [
    text(size: 26pt, weight: "bold", fill: primary-color, name)
    #v(6pt)
    text(size: 10pt, fill: muted-color, contacts.join("  |  "))
  ])
  #v(4pt)
  #rule(width: 100%, thickness: 1.5pt, stroke: primary-color)
  #v(16pt)
}

// Section title with rule
#let section-title(title) = {
  text(size: 12pt, weight: "bold", fill: primary-color, upper(title))
  #v(2pt)
  #rule(width: 100%, thickness: 1pt, stroke: line-color)
  #v(10pt)
}

// Entry header for experience/education
#let entry-header(title, org, date, location) = {
  table(
    columns: (1fr, auto),
    [
      [text(weight: "bold", title), text(size: 9.5pt, fill: muted-color, date)],
      [text(fill: secondary-color, org), text(size: 9.5pt, fill: muted-color, location)],
    ]
  )
  #v(4pt)
}

// Description text
#let entry-desc(desc) = {
  text(desc)
  #v(6pt)
}

// Bullet item
#let bullet-item(item) = {
  - #item
}

// Skill entry
#let skill-entry(name, level) = {
  table(
    columns: (1fr, auto),
    [text(weight: "medium", name), text(size: 9pt, fill: muted-color, level)],
  )
  #v(2pt)
}

// Language entry
#let language-entry(name, proficiency) = {
  table(
    columns: (1fr, auto),
    [text(weight: "medium", name), text(size: 9.5pt, fill: muted-color, proficiency)],
  )
  #v(3pt)
}

// Project entry
#let project-entry(name, desc, techs, links) = {
  text(weight: "bold", name)
  #v(2pt)
  text(size: 10pt, desc)
  #v(4pt)
  text(size: 9.5pt, fill: muted-color, "Technologies: " + techs.join(", "))
  #if links.len() > 0 {
    #v(4pt)
    #for link in links {
      #link
      #h(12pt)
    }
  }
  #v(10pt)
}

// Certification entry
#let cert-entry(name, issuer, date, expiry, url, cred-id) = {
  entry-header(name, issuer, date, "")
  #if expiry != "" {
    text(size: 9.5pt, fill: muted-color, "Expires: " + expiry)
    #v(3pt)
  }
  #if url != "" {
    #link(url)[Verify]
    #if cred-id != "" { #h(12pt) text(size: 9pt, fill: muted-color, "ID: " + cred-id) }
  } #else if cred-id != "" {
    text(size: 9pt, fill: muted-color, "ID: " + cred-id)
  }
  #v(10pt)
}

// ============================================================
// MAIN RENDER FUNCTION - Called with injected data
// ============================================================

#let render-cv(data) = {
  // Extract personal info
  let personal = data.personal
  let contacts = (
    personal.email,
    personal.phone,
    personal.location,
    personal.linkedin,
    personal.github,
    personal.portfolio
  ).filter(x => x != "")
  
  // Header
  header(personal.full_name, contacts)
  
  // Summary
  #if personal.summary != "" {
    section-title("Summary")
    text(personal.summary)
    #v(16pt)
  }
  
  // Experience
  #if data.experience.len() > 0 {
    section-title("Experience")
    #for exp in data.experience {
      entry-header(exp.title, exp.company, exp.date, exp.location)
      entry-desc(exp.description)
      #if exp.skills.len() > 0 {
        text(size: 9.5pt, fill: muted-color, "Key Skills: " + exp.skills.join(", "))
        #v(4pt)
      }
      #v(10pt)
    }
  }
  
  // Education
  #if data.education.len() > 0 {
    section-title("Education")
    #for edu in data.education {
      entry-header(edu.degree, edu.institution, edu.date, edu.location)
      #if edu.field != "" {
        text(size: 10pt, fill: secondary-color, edu.field)
        #v(3pt)
      }
      #if edu.grade != "" {
        text(size: 9.5pt, fill: muted-color, "Grade: " + edu.grade)
        #v(3pt)
      }
      #if edu.description != "" {
        #v(6pt)
        entry-desc(edu.description)
      }
      #v(10pt)
    }
  }
  
  // Skills
  #if data.skills.len() > 0 {
    section-title("Skills")
    #let grouped = data.skills.group(x => x.category ?? "General")
    #for (cat, skills) in grouped {
      #if cat != "General" {
        text(size: 11pt, weight: "bold", fill: primary-color, cat)
        #v(4pt)
      }
      #for skill in skills {
        skill-entry(skill.name + skill.level.map(l => " (" + l + ")").or(""), "")
      }
      #v(6pt)
    }
  }
  
  // Languages
  #if data.languages.len() > 0 {
    section-title("Languages")
    #for lang in data.languages {
      language-entry(lang.name, lang.proficiency)
    }
    #v(6pt)
  }
  
  // Projects
  #if data.projects.len() > 0 {
    section-title("Projects")
    #for proj in data.projects {
      project-entry(proj.name, proj.description, proj.technologies, proj.links)
    }
  }
  
  // Certifications
  #if data.certifications.len() > 0 {
    section-title("Certifications")
    #for cert in data.certifications {
      cert-entry(cert.name, cert.issuer, cert.date, cert.expiry, cert.url, cert.credential_id)
    }
  }
}

// The data structure expected:
// data = (
//   personal: (
//     full_name: "",
//     email: "",
//     phone: "",
//     location: "",
//     linkedin: "",
//     github: "",
//     portfolio: "",
//     summary: "",
//   ),
//   experience: (
//     (title: "", company: "", location: "", date: "", description: "", skills: ()), ...
//   ),
//   education: (
//     (degree: "", institution: "", location: "", date: "", field: "", grade: "", description: ""), ...
//   ),
//   skills: (
//     (name: "", category: "", level: ""), ...
//   ),
//   languages: (
//     (name: "", proficiency: ""), ...
//   ),
//   projects: (
//     (name: "", description: "", technologies: (), links: ()), ...
//   ),
//   certifications: (
//     (name: "", issuer: "", date: "", expiry: "", url: "", credential_id: ""), ...
//   ),
// )